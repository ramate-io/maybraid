//! Explicit dense-table reservation census, using the pinned Bevy allocation invariant.

use bevy::ecs::change_detection::Tick;
use bevy::prelude::*;
use serde::Serialize;
use std::collections::HashMap;

use crate::PayloadBytes;

const TABLE_LIMIT: usize = 16;
const TABLE_LAYOUT_LIMIT: usize = 16;
const COMPONENT_LIMIT: usize = 32;

/// Selected allocated element storage, not allocator or complete ECS heap bytes.
/// Component data overlaps the existing live inline-component census. Ticks and
/// entity IDs are separate buffers; nested allocations and metadata are excluded.
#[derive(Debug, Default, Serialize)]
pub struct DenseTableCensus {
	pub empty: TableOccupancyCensus,
	pub partial: TableOccupancyCensus,
	pub full: TableOccupancyCensus,
	pub component_data: PayloadBytes,
	pub change_ticks: PayloadBytes,
	pub entity_ids: PayloadBytes,
	pub unused_component_data_bytes: usize,
	/// Missing layouts make component-data totals incomplete, never assumed zero.
	pub unattributed_columns: usize,
	pub largest_tables: Vec<TableCapacityCensus>,
	pub largest_components: Vec<DenseComponentCapacityCensus>,
}

/// Mutually exclusive partitions of the all-table totals. Zero-row/zero-capacity
/// tables are empty, not full. Excludes allocator overhead, changed-by buffers,
/// archetype metadata and nested component allocations.
#[derive(Debug, Default, Serialize)]
pub struct TableOccupancyCensus {
	pub tables: usize,
	pub live_rows: usize,
	pub row_capacity: usize,
	pub component_data: PayloadBytes,
	pub change_ticks: PayloadBytes,
	pub entity_ids: PayloadBytes,
}

#[derive(Debug, Serialize)]
pub struct ComponentLayoutCensus {
	pub component_id: usize,
	pub name: String,
	/// Inline allocation stride, excluding pointed-to Vec/Arc/Box payloads.
	pub inline_bytes_per_row: usize,
	pub alignment: usize,
}

#[derive(Debug, Serialize)]
pub struct TableCapacityCensus {
	/// Identity within this World only.
	pub table_id: usize,
	pub live_rows: usize,
	pub row_capacity: usize,
	pub column_count: usize,
	pub attributed_columns: usize,
	pub component_data: PayloadBytes,
	pub change_ticks: PayloadBytes,
	pub entity_ids: PayloadBytes,
	/// Largest 16 inline layouts; totals above cover all attributed columns.
	pub largest_layouts: Vec<ComponentLayoutCensus>,
	pub omitted_layouts: usize,
}

impl TableCapacityCensus {
	fn selected_capacity_bytes(&self) -> usize {
		self.component_data.capacity_bytes
			+ self.change_ticks.capacity_bytes
			+ self.entity_ids.capacity_bytes
	}
}

#[derive(Debug, Serialize)]
pub struct DenseComponentCapacityCensus {
	pub component_id: usize,
	pub name: String,
	pub inline_bytes_per_row: usize,
	pub alignment: usize,
	pub tables: usize,
	pub live_rows: usize,
	pub reserved_rows: usize,
	pub component_data: PayloadBytes,
}

impl DenseTableCensus {
	/// No raw component dereference, mutation, shrink, or recurring scan. Bevy's
	/// Table::capacity equals column capacity outside an in-progress allocation;
	/// this immutable World borrow observes tables after that allocation completes.
	pub(crate) fn capture(world: &World) -> Self {
		let mut columns_by_table = HashMap::new();
		for archetype in world.archetypes().iter() {
			// Sparse-set archetypes can share a table. Attribute it exactly once.
			columns_by_table
				.entry(archetype.table_id().as_usize())
				.or_insert_with(|| archetype.table_components().collect::<Vec<_>>());
		}
		let mut result = Self::default();
		let mut components: HashMap<usize, DenseComponentCapacityCensus> = HashMap::new();
		for (table_id, table) in world.storages().tables.iter().enumerate() {
			let live_rows = table.entities().len();
			let row_capacity = table.capacity();
			let mut layouts = Vec::new();
			let mut bytes_per_row = 0;
			if let Some(ids) = columns_by_table.get(&table_id) {
				for id in ids {
					let Some(info) = world.components().get_info(*id) else { continue };
					if table.get_column(*id).is_none() {
						continue;
					}
					let stride = info.layout().size();
					bytes_per_row += stride;
					layouts.push(ComponentLayoutCensus {
						component_id: id.index(),
						name: info.name().to_string(),
						inline_bytes_per_row: stride,
						alignment: info.layout().align(),
					});
					let component = components.entry(id.index()).or_insert_with(|| {
						DenseComponentCapacityCensus {
							component_id: id.index(),
							name: info.name().to_string(),
							inline_bytes_per_row: stride,
							alignment: info.layout().align(),
							tables: 0,
							live_rows: 0,
							reserved_rows: 0,
							component_data: PayloadBytes::default(),
						}
					});
					component.tables += 1;
					component.live_rows += live_rows;
					component.reserved_rows += row_capacity;
					component.component_data.logical_bytes += live_rows * stride;
					component.component_data.capacity_bytes += row_capacity * stride;
				}
			}
			let attributed_columns = layouts.len();
			result.unattributed_columns +=
				table.component_count().saturating_sub(attributed_columns);
			let component_data = PayloadBytes {
				logical_bytes: live_rows * bytes_per_row,
				capacity_bytes: row_capacity * bytes_per_row,
			};
			// Pinned Bevy Column owns distinct added/changed Tick arrays at the
			// table capacity, including columns whose component data is zero-sized.
			let tick_bytes_per_row = table.component_count() * 2 * size_of::<Tick>();
			let change_ticks = PayloadBytes {
				logical_bytes: live_rows * tick_bytes_per_row,
				capacity_bytes: row_capacity * tick_bytes_per_row,
			};
			let entity_ids = PayloadBytes {
				logical_bytes: live_rows * size_of::<Entity>(),
				capacity_bytes: row_capacity * size_of::<Entity>(),
			};
			let partition = if live_rows == 0 {
				&mut result.empty
			} else if live_rows < row_capacity {
				&mut result.partial
			} else {
				&mut result.full
			};
			partition.tables += 1;
			partition.live_rows += live_rows;
			partition.row_capacity += row_capacity;
			partition.component_data.add(component_data);
			partition.change_ticks.add(change_ticks);
			partition.entity_ids.add(entity_ids);
			result.component_data.logical_bytes += component_data.logical_bytes;
			result.component_data.capacity_bytes += component_data.capacity_bytes;
			result.change_ticks.logical_bytes += change_ticks.logical_bytes;
			result.change_ticks.capacity_bytes += change_ticks.capacity_bytes;
			result.entity_ids.logical_bytes += entity_ids.logical_bytes;
			result.entity_ids.capacity_bytes += entity_ids.capacity_bytes;
			layouts.sort_by(|a, b| {
				b.inline_bytes_per_row
					.cmp(&a.inline_bytes_per_row)
					.then_with(|| a.component_id.cmp(&b.component_id))
			});
			let omitted_layouts = layouts.len().saturating_sub(TABLE_LAYOUT_LIMIT);
			layouts.truncate(TABLE_LAYOUT_LIMIT);
			result.largest_tables.push(TableCapacityCensus {
				table_id,
				live_rows,
				row_capacity,
				column_count: table.component_count(),
				attributed_columns,
				component_data,
				change_ticks,
				entity_ids,
				largest_layouts: layouts,
				omitted_layouts,
			});
		}
		result.unused_component_data_bytes =
			result.component_data.capacity_bytes - result.component_data.logical_bytes;
		result.largest_tables.sort_by(|a, b| {
			b.selected_capacity_bytes()
				.cmp(&a.selected_capacity_bytes())
				.then_with(|| a.table_id.cmp(&b.table_id))
		});
		result.largest_tables.truncate(TABLE_LIMIT);
		result.largest_components = components.into_values().collect();
		result.largest_components.sort_by(|a, b| {
			b.component_data
				.capacity_bytes
				.cmp(&a.component_data.capacity_bytes)
				.then_with(|| a.component_id.cmp(&b.component_id))
		});
		result.largest_components.truncate(COMPONENT_LIMIT);
		result
	}
}

#[cfg(test)]
mod tests {
	use super::*;

	#[derive(Component)]
	struct Wide([u8; 256]);
	#[derive(Component)]
	struct Zero;
	#[derive(Component)]
	#[component(storage = "SparseSet")]
	struct Sparse;

	#[derive(Component)]
	struct Heap(Vec<u8>);

	#[test]
	fn dense_layout_reservation_excludes_nested_vec_allocations() -> anyhow::Result<()> {
		let mut world = World::new();
		let entity = world.spawn(Heap(vec![0; 65_536])).id();
		let component_id = world
			.components()
			.component_id::<Heap>()
			.ok_or_else(|| anyhow::anyhow!("heap component"))?
			.index();
		let census = DenseTableCensus::capture(&world);
		let component = census
			.largest_components
			.iter()
			.find(|c| c.component_id == component_id)
			.ok_or_else(|| anyhow::anyhow!("heap inline census"))?;
		assert_eq!(component.component_data.logical_bytes, size_of::<Heap>());
		assert_eq!(
			component.component_data.capacity_bytes,
			component.reserved_rows * size_of::<Heap>()
		);
		assert_eq!(
			world.get::<Heap>(entity).ok_or_else(|| anyhow::anyhow!("heap value"))?.0.len(),
			65_536
		);
		assert!(
			component.component_data.logical_bytes < 65_536,
			"the pointed-to Vec data is outside dense inline storage"
		);
		Ok(())
	}

	#[test]
	fn partial_and_empty_tables_keep_reservation_without_double_counting_sparse_archetypes(
	) -> anyhow::Result<()> {
		let mut world = World::new();
		let entities: Vec<_> = (0..65).map(|_| world.spawn((Wide([0; 256]), Zero)).id()).collect();
		for entity in entities.iter().step_by(2) {
			world.entity_mut(*entity).insert(Sparse);
		}
		let retained = entities[0];
		assert_eq!(
			world.get::<Wide>(retained).ok_or_else(|| anyhow::anyhow!("wide data"))?.0[0],
			0
		);
		let table_id = world.entity(retained).archetype().table_id();
		let capacity = world
			.storages()
			.tables
			.get(table_id)
			.ok_or_else(|| anyhow::anyhow!("table"))?
			.capacity();
		let wide_id = world
			.components()
			.component_id::<Wide>()
			.ok_or_else(|| anyhow::anyhow!("wide component"))?
			.index();
		let sparse_id = world
			.components()
			.component_id::<Sparse>()
			.ok_or_else(|| anyhow::anyhow!("sparse component"))?
			.index();
		for entity in &entities[1..] {
			world.despawn(*entity);
		}
		let partial = DenseTableCensus::capture(&world);
		let table = partial
			.largest_tables
			.iter()
			.find(|t| t.table_id == table_id.as_usize())
			.ok_or_else(|| anyhow::anyhow!("partial table row"))?;
		assert_eq!(table.live_rows, 1);
		assert_eq!(table.row_capacity, capacity);
		assert_eq!(table.column_count, 2);
		assert_eq!(table.attributed_columns, 2);
		assert_eq!(
			table.component_data,
			PayloadBytes { logical_bytes: 256, capacity_bytes: capacity * 256 }
		);
		assert_eq!(table.change_ticks.capacity_bytes, capacity * 2 * 2 * size_of::<Tick>());
		assert_eq!(table.entity_ids.capacity_bytes, capacity * size_of::<Entity>());
		let wide = partial
			.largest_components
			.iter()
			.find(|c| c.component_id == wide_id)
			.ok_or_else(|| anyhow::anyhow!("wide reservation"))?;
		assert_eq!(wide.tables, 1, "sparse archetypes share one dense table");
		assert_eq!(wide.component_data.capacity_bytes, capacity * 256);
		assert!(!partial.largest_components.iter().any(|c| c.component_id == sparse_id));
		assert_eq!(partial.unattributed_columns, 0);
		assert!(partial.partial.component_data.capacity_bytes >= capacity * 256);
		assert_eq!(
			partial.component_data.capacity_bytes,
			partial.empty.component_data.capacity_bytes
				+ partial.partial.component_data.capacity_bytes
				+ partial.full.component_data.capacity_bytes
		);
		world.despawn(retained);
		let empty = DenseTableCensus::capture(&world);
		let table = empty
			.largest_tables
			.iter()
			.find(|t| t.table_id == table_id.as_usize())
			.ok_or_else(|| anyhow::anyhow!("empty retained table"))?;
		assert_eq!(table.live_rows, 0);
		assert_eq!(table.component_data.logical_bytes, 0);
		assert_eq!(table.component_data.capacity_bytes, capacity * 256);
		assert_eq!(table.change_ticks.logical_bytes, 0);
		assert_eq!(table.row_capacity, capacity);
		assert!(empty.empty.component_data.capacity_bytes >= capacity * 256);
		assert_eq!(empty.empty.component_data.logical_bytes, 0);
		Ok(())
	}
}
