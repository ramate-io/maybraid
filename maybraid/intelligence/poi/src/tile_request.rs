//! Nonblocking distant tile requests. Generation stays on the HCSG worker.

use bevy::math::IVec2;
use lod::hcsg::{
	Busy, HcsgClass, HcsgDemand, HcsgStorage, HcsgTicket, PublishedReadError, PublishedRegion,
	TicketPoll, TicketRequestError,
};

use crate::{xz_tile, xz_tile_region, PoiEntry, PoiInterests, PoiOrigin, PoiSet};

/// Immutable identity of one distant probe; filtering preserves POI id order.
#[derive(Clone, Debug, PartialEq)]
pub struct PoiTileQuery {
	pub tile: IVec2,
	pub tile_size: f32,
	pub center_y: f32,
	pub interests: PoiInterests,
}

/// Available entries can accompany every state, including Pending or Busy.
/// Only Ready/ReadyEmpty certify generation and published-read completion.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PoiTilePoll {
	Pending,
	Capacity,
	Ready,
	ReadyEmpty,
	Stale,
	Failed,
}

/// One consumer's available readout and independent generation ownership.
/// Dropping it releases subscriptions. `allow_request` gates new admission only;
/// reads and existing work keep progressing during a generation retry cooldown.
pub trait DistantTileRequest: Send + Sync + 'static {
	/// Release any partial group admission before retrying a saturated pool.
	fn release(&mut self) {}

	fn poll(
		&mut self,
		storage: &HcsgStorage,
		demand: &HcsgDemand,
		out: &mut Vec<PoiEntry>,
		allow_request: bool,
	) -> Result<PoiTilePoll, Busy>;
}

/// Request one existing PoiSet scheme; no second queue or spatial cache.
pub struct PoiTileRequest<S: PoiOrigin> {
	query: PoiTileQuery,
	ticket: Option<HcsgTicket<PoiSet<S>>>,
	published: Option<PublishedRegion<PoiSet<S>>>,
}

impl<S: PoiOrigin> PoiTileRequest<S> {
	pub fn new(query: PoiTileQuery) -> Self {
		Self { query, ticket: None, published: None }
	}
}

impl<S: PoiOrigin> DistantTileRequest for PoiTileRequest<S> {
	fn release(&mut self) {
		self.ticket = None;
	}

	fn poll(
		&mut self,
		storage: &HcsgStorage,
		demand: &HcsgDemand,
		out: &mut Vec<PoiEntry>,
		allow_request: bool,
	) -> Result<PoiTilePoll, Busy> {
		out.clear();
		let epoch = demand.try_epoch()?;
		let status = self.poll_available(storage, demand, out, allow_request);
		match demand.try_epoch() {
			Ok(current) if current == epoch => {
				out.sort_unstable_by_key(|poi| poi.id);
				out.dedup_by_key(|poi| poi.id);
				status
			}
			Ok(_) => {
				out.clear();
				self.ticket = None;
				self.published = None;
				Ok(PoiTilePoll::Stale)
			}
			Err(busy) => {
				out.clear();
				Err(busy)
			}
		}
	}
}

impl<S: PoiOrigin> PoiTileRequest<S> {
	fn poll_available(
		&mut self,
		storage: &HcsgStorage,
		demand: &HcsgDemand,
		out: &mut Vec<PoiEntry>,
		allow_request: bool,
	) -> Result<PoiTilePoll, Busy> {
		out.clear();
		let Some(region) =
			xz_tile_region(self.query.tile, self.query.tile_size, self.query.center_y)
		else {
			return Ok(PoiTilePoll::Failed);
		};
		if self.published.as_ref().is_some_and(|readout| !readout.is_for(storage, demand)) {
			self.ticket = None;
			self.published = None;
		}
		let published = self
			.published
			.get_or_insert_with(|| PublishedRegion::new(storage.clone(), demand.clone(), region));
		let cache_complete = match published.try_read() {
			Ok(read) => {
				for (_, entry) in published.entries() {
					self.query.extend_matching(&entry.value, out);
				}
				read.complete
			}
			Err(PublishedReadError::Busy) => false,
			Err(PublishedReadError::Stale) => {
				self.ticket = None;
				return Ok(PoiTilePoll::Stale);
			}
		};
		if self.ticket.is_none() && allow_request {
			self.ticket = Some(match demand.try_request::<PoiSet<S>>(region, HcsgClass::Near) {
				Ok(ticket) => ticket,
				Err(TicketRequestError::Busy) => return Err(Busy),
				Err(TicketRequestError::Capacity) => return Ok(PoiTilePoll::Capacity),
				Err(TicketRequestError::InvalidRegion) => return Ok(PoiTilePoll::Failed),
			});
		}
		let Some(ticket) = self.ticket.as_mut() else {
			out.sort_unstable_by_key(|poi| poi.id);
			out.dedup_by_key(|poi| poi.id);
			return Ok(PoiTilePoll::Pending);
		};
		let progress = ticket.try_poll(storage)?;
		match progress {
			TicketPoll::Pending | TicketPoll::Ready => {
				for set in ticket.values() {
					self.query.extend_matching(set, out);
				}
			}
			TicketPoll::Stale => {
				self.ticket = None;
				return Ok(PoiTilePoll::Pending);
			}
			TicketPoll::Failed => {
				self.ticket = None;
				return Ok(PoiTilePoll::Failed);
			}
		}
		out.sort_unstable_by_key(|poi| poi.id);
		out.dedup_by_key(|poi| poi.id);
		Ok(if progress == TicketPoll::Ready && cache_complete {
			if out.is_empty() {
				PoiTilePoll::ReadyEmpty
			} else {
				PoiTilePoll::Ready
			}
		} else {
			PoiTilePoll::Pending
		})
	}
}

impl PoiTileQuery {
	fn extend_matching<S: PoiOrigin>(&self, set: &PoiSet<S>, out: &mut Vec<PoiEntry>) {
		out.extend(set.pois.iter().copied().filter(|poi| {
			self.interests.contains(poi.kind) && xz_tile(poi.position, self.tile_size) == self.tile
		}));
	}
}
