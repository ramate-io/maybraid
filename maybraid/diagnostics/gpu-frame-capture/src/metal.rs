use bevy::{
	prelude::*,
	render::{
		Render, RenderApp, RenderSystems,
		render_resource::{CachedPipelineState, PipelineCache},
		renderer::{RenderAdapterInfo, RenderDevice},
	},
};
use objc2::{rc::Retained, runtime::ProtocolObject};
use objc2_foundation::{NSString, NSURL};
use objc2_metal::{MTLCaptureDescriptor, MTLCaptureDestination, MTLCaptureManager, MTLDevice};
use serde_json::json;
use std::{
	fs::{self, File, OpenOptions},
	io::{Seek, SeekFrom, Write},
	os::unix::fs::{OpenOptionsExt, PermissionsExt},
	path::PathBuf,
	time::{SystemTime, UNIX_EPOCH},
};

pub(crate) fn install(app: &mut App) {
	if std::env::var_os("MAYBRAID_GPU_CAPTURE_OUTPUT").is_none()
		&& std::env::var_os("MAYBRAID_GPU_CAPTURE_TRIGGER").is_none()
	{
		return;
	}
	if let Some(render) = app.get_sub_app_mut(RenderApp) {
		render.init_resource::<Initialized>();
		render.add_systems(
			Render,
			(
				begin
					.before(RenderSystems::ExtractCommands)
					.before(RenderSystems::PrepareAssets),
				end.after(RenderSystems::Render).before(RenderSystems::Cleanup),
			),
		);
	} else {
		eprintln!("[gpu-frame-capture] no render app");
	}
}

#[derive(Resource, Default)]
struct Initialized(bool);

#[derive(Clone, Copy)]
struct PipelineSnapshot {
	waiting: usize,
	failed: usize,
}
impl PipelineSnapshot {
	fn from_cache(cache: &PipelineCache) -> Self {
		Self {
			waiting: cache.waiting_pipelines().count(),
			failed: cache
				.pipelines()
				.filter(|pipeline| matches!(pipeline.state, CachedPipelineState::Err(_)))
				.count(),
		}
	}
}

struct Capture {
	output: PathBuf,
	trigger: PathBuf,
	claimed: PathBuf,
	status: File,
	request: String,
	frame: u64,
	requested_frame: Option<u64>,
	started_frame: Option<u64>,
	stopped_frame: Option<u64>,
	device_identity: Option<String>,
	backend: Option<String>,
	pipelines: Option<PipelineSnapshot>,
	session: Option<Session>,
	done: bool,
}

impl Capture {
	fn from_env() -> Result<Option<Self>, String> {
		let output = std::env::var_os("MAYBRAID_GPU_CAPTURE_OUTPUT");
		let trigger = std::env::var_os("MAYBRAID_GPU_CAPTURE_TRIGGER");
		let (output, trigger) = match (output, trigger) {
			(None, None) => return Ok(None),
			(Some(output), Some(trigger)) => (PathBuf::from(output), PathBuf::from(trigger)),
			_ => {
				return Err(
					"set both MAYBRAID_GPU_CAPTURE_OUTPUT and MAYBRAID_GPU_CAPTURE_TRIGGER".into(),
				);
			}
		};
		Self::from_paths(output, trigger).map(Some)
	}

	fn from_paths(output: PathBuf, trigger: PathBuf) -> Result<Self, String> {
		if !output.is_absolute() || !trigger.is_absolute() || output == trigger {
			return Err("output and trigger must be distinct absolute paths".into());
		}
		if output.extension().is_none_or(|ext| ext != "gputrace") {
			return Err("output must end in .gputrace".into());
		}
		let parent = output.parent().ok_or("output has no parent")?;
		if trigger.parent() != Some(parent)
			|| parent.canonicalize().map_err(|e| e.to_string())? != parent
		{
			return Err("output and trigger must share a canonical private run directory".into());
		}
		if fs::metadata(parent).map_err(|e| e.to_string())?.permissions().mode() & 0o077 != 0 {
			return Err("run directory must be private (mode 0700)".into());
		}
		for path in [&output, &trigger] {
			match fs::symlink_metadata(path) {
				Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
				_ => return Err(format!("refusing existing/inaccessible path {}", path.display())),
			}
		}
		if matches!(
			trigger.file_name().and_then(|name| name.to_str()),
			Some("capture-status.json" | "capture.trigger.claimed")
		) {
			return Err("trigger uses a reserved sidecar path".into());
		}
		let claimed = parent.join("capture.trigger.claimed");
		if claimed.exists() {
			return Err("stale claimed trigger".into());
		}
		let status = OpenOptions::new()
			.write(true)
			.create_new(true)
			.mode(0o600)
			.open(parent.join("capture-status.json"))
			.map_err(|e| e.to_string())?;
		let request = format!(
			"{}-{}",
			std::process::id(),
			SystemTime::now()
				.duration_since(UNIX_EPOCH)
				.map_err(|e| e.to_string())?
				.as_nanos()
		);
		let mut capture = Self {
			output,
			trigger,
			claimed,
			status,
			request,
			frame: 0,
			requested_frame: None,
			started_frame: None,
			stopped_frame: None,
			device_identity: None,
			backend: None,
			pipelines: None,
			session: None,
			done: false,
		};
		capture.record("armed", None)?;
		Ok(capture)
	}

	fn record(&mut self, state: &str, error: Option<&str>) -> Result<(), String> {
		let value = json!({"schema": 1, "state": state, "request_id": self.request,
			"pid": std::process::id(), "backend": self.backend, "output": self.output,
			"requested_render_frame": self.requested_frame, "started_render_frame": self.started_frame,
			"stopped_render_frame": self.stopped_frame, "device_identity": self.device_identity,
			"pipeline_pending_count": self.pipelines.map(|status| status.waiting),
			"pipeline_failed_count": self.pipelines.map(|status| status.failed),
			"scope": "one render update; device background work may also appear", "error": error});
		let bytes = serde_json::to_vec_pretty(&value).map_err(|e| e.to_string())?;
		self.status.seek(SeekFrom::Start(0)).map_err(|e| e.to_string())?;
		self.status.set_len(0).map_err(|e| e.to_string())?;
		self.status
			.write_all(&bytes)
			.and_then(|()| self.status.flush())
			.map_err(|e| e.to_string())
	}

	fn fail(&mut self, error: String) {
		// A status-write failure after start must stop ownership without later becoming complete.
		if let Some(mut session) = self.session.take() {
			session.stop();
			self.stopped_frame = Some(self.frame);
		}
		self.done = true;
		eprintln!("[gpu-frame-capture] {error}");
		if let Err(error) = self.record("failed", Some(&error)) {
			eprintln!("[gpu-frame-capture] status: {error}");
		}
	}

	fn begin(
		&mut self,
		device_arg: Option<&RenderDevice>,
		pipelines: Option<PipelineSnapshot>,
	) -> Result<(), String> {
		self.frame += 1;
		if self.done || self.session.is_some() {
			return Ok(());
		}
		let metadata = match fs::symlink_metadata(&self.trigger) {
			Err(error) if error.kind() == std::io::ErrorKind::NotFound => return Ok(()),
			result => result.map_err(|e| e.to_string())?,
		};
		self.requested_frame = Some(self.frame);
		self.pipelines = pipelines;
		let pipelines =
			pipelines.ok_or("pipeline cache unavailable at trigger; refusing capture")?;
		if pipelines.waiting != 0 || pipelines.failed != 0 {
			return Err(format!(
				"pipelines not settled: {} waiting, {} failed; refusing capture",
				pipelines.waiting, pipelines.failed
			));
		}
		if !metadata.file_type().is_file() {
			return Err("trigger must be a regular file".into());
		}
		// Hard-link creation is an atomic no-replace claim inside our private run directory.
		fs::hard_link(&self.trigger, &self.claimed).map_err(|e| format!("claim trigger: {e}"))?;
		fs::remove_file(&self.trigger).map_err(|e| e.to_string())?;
		if fs::read_to_string(&self.claimed).map_err(|e| e.to_string())?.trim() != self.request {
			return Err("stale or foreign trigger request_id".into());
		}
		if self.output.try_exists().map_err(|e| e.to_string())? {
			return Err("output already exists".into());
		}
		let wgpu_device = device_arg.ok_or("no render device")?.wgpu_device().clone();
		let device = metal_device(device_arg)?;
		self.device_identity = Some(format!("{:p}", Retained::as_ptr(&device)));
		// The manager is process-global. Never stop a capture that another caller started.
		let manager = unsafe { MTLCaptureManager::sharedCaptureManager() };
		if manager.isCapturing() {
			return Err("capture manager already active".into());
		}
		if !manager.supportsDestination(MTLCaptureDestination::GPUTraceDocument) {
			return Err("GPUTraceDocument unsupported (launch with MTL_CAPTURE_ENABLED=1)".into());
		}
		let descriptor = MTLCaptureDescriptor::new();
		let path = self.output.to_str().ok_or("output path must be UTF-8")?;
		let url = NSURL::fileURLWithPath(&NSString::from_str(path));
		descriptor.setDestination(MTLCaptureDestination::GPUTraceDocument);
		descriptor.setOutputURL(Some(&url));
		// SAFETY: the retained protocol object is an actual MTLDevice from the active wgpu device.
		unsafe { descriptor.setCaptureObject(Some((&*device).as_ref())) };
		manager
			.startCaptureWithDescriptor_error(&descriptor)
			.map_err(|e| format!("start capture: {e}"))?;
		self.session = Some(Session { manager, device, wgpu_device, active: true });
		self.started_frame = Some(self.frame);
		self.record("capturing", None)
	}

	fn end(&mut self, device: Option<&RenderDevice>) -> Result<(), String> {
		let Some(mut session) = self.session.take() else {
			return Ok(());
		};
		let same_device = device.is_some_and(|active| active.wgpu_device() == &session.wgpu_device)
			&& metal_device(device)
				.is_ok_and(|active| Retained::as_ptr(&active) == Retained::as_ptr(&session.device));
		session.stop();
		self.stopped_frame = Some(self.frame);
		self.done = true;
		if !same_device {
			return Err("active Metal device changed during capture; discard trace".into());
		}
		self.record("complete", None)
	}
}

impl Drop for Capture {
	fn drop(&mut self) {
		if let Some(mut session) = self.session.take() {
			session.stop();
			self.stopped_frame = Some(self.frame);
			let _ = self
				.record("aborted", Some("orderly teardown during owned capture; discard trace"));
		}
	}
}

struct Session {
	manager: Retained<MTLCaptureManager>,
	device: Retained<ProtocolObject<dyn MTLDevice>>,
	wgpu_device: wgpu::Device,
	active: bool,
}
impl Session {
	fn stop(&mut self) {
		if self.active {
			self.manager.stopCapture();
			self.active = false;
		}
	}
}
impl Drop for Session {
	fn drop(&mut self) {
		self.stop();
	}
}

fn metal_device(
	device: Option<&RenderDevice>,
) -> Result<Retained<ProtocolObject<dyn MTLDevice>>, String> {
	let device = device.ok_or("no active render device")?;
	// SAFETY: only clone/retain the raw device while the scoped HAL guard is alive.
	// We do not mutate/destroy it or keep a borrowed HAL pointer after the guard drops.
	let retained = unsafe { device.wgpu_device().as_hal::<wgpu::hal::api::Metal>() }
		.ok_or("active render backend is not Metal")?
		.raw_device()
		.clone();
	Ok(retained)
}

fn begin(world: &mut World) {
	if !world.resource::<Initialized>().0 {
		world.resource_mut::<Initialized>().0 = true;
		// Initialize on the render thread: Objective-C objects stay in a non-Send resource.
		match Capture::from_env() {
			Ok(Some(capture)) => world.insert_non_send(capture),
			Ok(None) => return,
			Err(error) => {
				eprintln!("[gpu-frame-capture] refused: {error}");
				return;
			}
		}
	}
	if !world.contains_non_send::<Capture>() {
		return;
	}
	let device = world.get_resource::<RenderDevice>().cloned();
	let backend = world
		.get_resource::<RenderAdapterInfo>()
		.map(|info| format!("{:?}", info.backend));
	let pipelines = world
		.get_non_send::<Capture>()
		.filter(|capture| capture.trigger.exists())
		.and_then(|_| world.get_resource::<PipelineCache>())
		.map(PipelineSnapshot::from_cache);
	let mut capture = world.non_send_mut::<Capture>();
	capture.backend = backend;
	if let Err(error) = capture.begin(device.as_ref(), pipelines) {
		capture.fail(error);
	}
}
fn end(world: &mut World) {
	if !world.contains_non_send::<Capture>() {
		return;
	}
	let device = world.get_resource::<RenderDevice>().cloned();
	let mut capture = world.non_send_mut::<Capture>();
	if let Err(error) = capture.end(device.as_ref()) {
		capture.fail(error);
	}
	let done = capture.done;
	drop(capture);
	if done {
		// Drop on the render thread; Bevy otherwise forgets non-Send data at main-thread shutdown.
		world.remove_non_send::<Capture>();
	}
}

#[cfg(test)]
mod tests {
	use crate::metal::{Capture, PipelineSnapshot};
	use std::{fs, os::unix::fs::PermissionsExt};

	#[test]
	fn refuses_stale_paths_and_duplicate_run() -> Result<(), Box<dyn std::error::Error>> {
		let directory = tempfile::tempdir()?;
		fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))?;
		let parent = directory.path().canonicalize()?;
		let output = parent.join("frame.gputrace");
		let trigger = parent.join("trigger");
		fs::write(&trigger, "stale")?;
		assert!(Capture::from_paths(output.clone(), trigger.clone()).is_err());
		fs::remove_file(&trigger)?;
		fs::write(&output, "existing")?;
		assert!(Capture::from_paths(output.clone(), trigger.clone()).is_err());
		fs::remove_file(&output)?;
		let capture = Capture::from_paths(output.clone(), trigger.clone())?;
		assert!(Capture::from_paths(output, trigger).is_err());
		assert_eq!(capture.frame, 0);
		Ok(())
	}

	#[test]
	fn claims_foreign_trigger_once_without_starting_capture()
	-> Result<(), Box<dyn std::error::Error>> {
		let directory = tempfile::tempdir()?;
		fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))?;
		let parent = directory.path().canonicalize()?;
		let trigger = parent.join("trigger");
		let mut capture = Capture::from_paths(parent.join("frame.gputrace"), trigger.clone())?;
		capture.begin(None, Some(PipelineSnapshot { waiting: 0, failed: 0 }))?;
		assert_eq!(capture.requested_frame, None);
		fs::write(&trigger, "foreign")?;
		assert!(capture.begin(None, Some(PipelineSnapshot { waiting: 0, failed: 0 })).is_err());
		assert!(!trigger.exists());
		assert!(capture.claimed.exists());
		assert!(capture.session.is_none());
		assert_eq!(capture.requested_frame, Some(2));
		assert_eq!(capture.started_frame, None);
		Ok(())
	}

	#[test]
	fn pending_pipelines_refuse_once_and_preserve_failed_status()
	-> Result<(), Box<dyn std::error::Error>> {
		let directory = tempfile::tempdir()?;
		fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o700))?;
		let parent = directory.path().canonicalize()?;
		let trigger = parent.join("trigger");
		let mut capture = Capture::from_paths(parent.join("frame.gputrace"), trigger.clone())?;
		fs::write(&trigger, &capture.request)?;
		let error = match capture.begin(None, Some(PipelineSnapshot { waiting: 1, failed: 0 })) {
			Err(error) => error,
			Ok(()) => return Err("pending pipeline unexpectedly accepted".into()),
		};
		capture.fail(error);
		capture.end(None)?;
		assert!(capture.done);
		assert!(trigger.exists());
		assert!(!capture.claimed.exists());
		assert!(capture.session.is_none());
		let status: serde_json::Value =
			serde_json::from_slice(&fs::read(parent.join("capture-status.json"))?)?;
		assert_eq!(status["state"], "failed");
		assert_eq!(status["pipeline_pending_count"], 1);
		Ok(())
	}

	#[test]
	fn refuses_relative_paths_and_shared_directory() -> Result<(), Box<dyn std::error::Error>> {
		assert!(Capture::from_paths("relative.gputrace".into(), "trigger".into()).is_err());
		let directory = tempfile::tempdir()?;
		fs::set_permissions(directory.path(), fs::Permissions::from_mode(0o755))?;
		let parent = directory.path().canonicalize()?;
		assert!(
			Capture::from_paths(parent.join("frame.gputrace"), parent.join("trigger")).is_err()
		);
		Ok(())
	}
}
