//! A synthetic `.TDT` file built from a short spec, for the importer tests.
//! Every byte is made here from the layout in `src/tdt/format.rs`; no byte of
//! a real save is ever committed.

use verticopolis_engine::tdt::format::{
    TDT_CLINIC_LIST_SIZE, TDT_ELEVATOR_BUILT_FIXED, TDT_ELEVATOR_CAR_BLOCK_SIZE,
    TDT_ELEVATOR_HEADER_SIZE, TDT_ELEVATOR_PER_FLOOR_SIZE, TDT_ELEVATOR_SLOTS, TDT_FINANCE_SIZE,
    TDT_FLOOR_COUNT, TDT_FLOOR_INDEX_ENTRIES, TDT_FLOOR_OFFSET, TDT_HEADER_SIZE, TDT_MAGIC,
    TDT_PARKING_SIZE, TDT_PERSON_RECORD_SIZE, TDT_RETAIL_RECORD_SIZE, TDT_RETAIL_SLOTS,
    TDT_STAIR_RECORD_SIZE, TDT_STAIR_SLOTS, TDT_STATS_SIZE, TDT_UNKNOWN_BYTES_SIZE,
    TDT_UNKNOWN_FLAGS_SIZE,
};

/// One tenant record. `left..right` in tiles; a negative type is a room
/// under construction.
#[derive(Clone, Default)]
pub struct Tenant {
    pub left: i64,
    pub right: i64,
    pub type_id: i64,
    pub status: u8,
    /// Byte 16: 0 Very Low to 3 High, 4 No Rate. `None` writes 2 (Average).
    pub rent_rate: Option<u8>,
    /// Byte 6, where the game keeps a shop's style.
    pub variant: u8,
    /// Byte 17, which the game leaves 0.
    pub byte17: u8,
}

pub fn tenant(left: i64, right: i64, type_id: i64) -> Tenant {
    Tenant {
        left,
        right,
        type_id,
        ..Tenant::default()
    }
}

#[derive(Clone, Default)]
pub struct Floor {
    /// The `.TDT` floor index, 0 to 119.
    pub index: i64,
    pub left_edge: Option<i64>,
    pub right_edge: Option<i64>,
    pub tenants: Vec<Tenant>,
    /// A forged tenant-count word.
    pub forge_count: Option<u16>,
}

pub fn floor(index: i64, tenants: Vec<Tenant>) -> Floor {
    Floor {
        index,
        tenants,
        ..Floor::default()
    }
}

#[derive(Clone, Default)]
pub struct Elevator {
    /// 0 express, 1 standard, 2 service.
    pub type_id: u8,
    pub cars: u8,
    pub x: i64,
    /// `.TDT` floor indexes.
    pub bottom: i64,
    pub top: i64,
    /// The floors it stops at; every floor of the span when `None`.
    pub serviced: Option<Vec<i64>>,
    pub car_homes: Option<Vec<i64>>,
    /// Per-floor entries to write whatever the span says.
    pub payload_floors: Option<i64>,
}

pub fn elevator(type_id: u8, cars: u8, x: i64, bottom: i64, top: i64) -> Elevator {
    Elevator {
        type_id,
        cars,
        x,
        bottom,
        top,
        ..Elevator::default()
    }
}

#[derive(Clone, Copy)]
pub struct Stair {
    /// 0 escalator, 1 stairs; 2 and 3 two stories, 4 and 5 three.
    pub type_id: u8,
    pub x: i64,
    pub floor: i64,
}

/// How a file sizes its built-shaft payloads and lays the blocks after them.
#[derive(Clone, Copy, Default, PartialEq, Eq)]
pub enum Layout {
    /// What the game writes: a standard or service shaft by its span, an
    /// express by its eligible floors.
    #[default]
    Game,
    /// Our 2.9.0 and earlier: every shaft by the floors it stops at.
    LegacyServiced,
    /// Our generation 1: every shaft by its span, express included.
    SpannedExpress,
    /// An express by the floors it stops at.
    StoppedExpress,
}

#[derive(Clone)]
pub struct Spec {
    pub magic: u16,
    pub level: u16,
    pub balance: i32,
    pub frame_time: u16,
    pub current_day: i32,
    pub view: (u16, u16),
    pub floors: Vec<Floor>,
    pub people: u32,
    pub include_retail: bool,
    pub retail_rows: usize,
    pub include_transports: bool,
    pub elevators: Vec<Option<Elevator>>,
    pub layout: Layout,
    /// Lay the blocks after the elevators the way our generations 1 and 2 did.
    pub generation_one_blocks: bool,
    pub stairs: Vec<Stair>,
    /// Fill built-shaft payloads with 0xFF, as a real save's unused slots are.
    pub ff_payload: bool,
    pub parking_connected: u16,
    pub trailing_zeros: usize,
    pub routing_tail: usize,
    pub truncate_at: Option<usize>,
}

impl Default for Spec {
    fn default() -> Self {
        Spec {
            magic: TDT_MAGIC as u16,
            level: 1,
            balance: 20_000,
            frame_time: 0,
            current_day: 0,
            view: (0, 0),
            floors: vec![],
            people: 0,
            include_retail: true,
            retail_rows: 0,
            include_transports: true,
            elevators: vec![],
            layout: Layout::Game,
            generation_one_blocks: false,
            stairs: vec![],
            ff_payload: false,
            parking_connected: 0,
            trailing_zeros: 0,
            routing_tail: 0,
            truncate_at: None,
        }
    }
}

/// A built shaft's payload: the fixed block, an entry per floor, the car block.
pub fn payload_size(floors: i64) -> usize {
    (TDT_ELEVATOR_BUILT_FIXED + TDT_ELEVATOR_PER_FLOOR_SIZE * floors + TDT_ELEVATOR_CAR_BLOCK_SIZE)
        as usize
}

/// The floors an express has an entry for: at or below the first floor, and
/// every sky lobby.
pub fn express_eligible(bottom: i64, top: i64) -> i64 {
    (bottom..=top)
        .filter(|raw| {
            let ours = raw - TDT_FLOOR_OFFSET;
            ours <= 1 || ours % 15 == 0
        })
        .count() as i64
}

struct W(Vec<u8>);

impl W {
    fn u8(&mut self, v: u8) {
        self.0.push(v);
    }
    fn u16(&mut self, v: u16) {
        self.0.extend(v.to_le_bytes());
    }
    fn i32(&mut self, v: i32) {
        self.0.extend(v.to_le_bytes());
    }
    fn fill(&mut self, n: usize, v: u8) {
        self.0.extend(std::iter::repeat_n(v, n));
    }
    fn pad(&mut self, n: usize) {
        self.fill(n, 0);
    }
    fn pad_to(&mut self, at: usize) {
        let n = at - self.0.len();
        self.pad(n);
    }
}

pub fn build(spec: &Spec) -> Vec<u8> {
    let mut w = W(vec![]);
    w.u16(spec.magic);
    w.u16(spec.level);
    w.i32(spec.balance);
    w.i32(0);
    w.i32(0);
    w.i32(0);
    w.u16(spec.frame_time);
    w.i32(spec.current_day);
    w.pad_to(0x26);
    w.u16(spec.view.0);
    w.u16(spec.view.1);
    w.pad_to(TDT_HEADER_SIZE);

    for index in 0..TDT_FLOOR_COUNT {
        let f = spec.floors.iter().find(|f| f.index == index);
        let tenants: &[Tenant] = f.map(|f| &f.tenants[..]).unwrap_or(&[]);
        let left = f
            .and_then(|f| f.left_edge)
            .unwrap_or(tenants.iter().map(|t| t.left).min().unwrap_or(0));
        let right = f
            .and_then(|f| f.right_edge)
            .unwrap_or(tenants.iter().map(|t| t.right).max().unwrap_or(0));
        w.u16(
            f.and_then(|f| f.forge_count)
                .unwrap_or(tenants.len() as u16),
        );
        w.u16(left as u16);
        w.u16(right as u16);
        for t in tenants {
            w.u16(t.left as u16);
            w.u16(t.right as u16);
            w.u8(t.type_id as u8);
            w.u8(t.status);
            w.u8(t.variant);
            w.pad(9);
            w.u8(t.rent_rate.unwrap_or(2));
            w.u8(t.byte17);
        }
        w.pad(TDT_FLOOR_INDEX_ENTRIES * 2);
    }

    w.i32(spec.people as i32);
    w.pad(spec.people as usize * TDT_PERSON_RECORD_SIZE);
    if !spec.include_retail {
        return finish(w.0, spec);
    }
    for slot in 0..TDT_RETAIL_SLOTS {
        w.u8(if slot < spec.retail_rows { 10 } else { 0xff });
        w.pad(TDT_RETAIL_RECORD_SIZE - 1);
    }
    if !spec.include_transports {
        return finish(w.0, spec);
    }

    let fill = if spec.ff_payload { 0xff } else { 0 };
    for slot in 0..TDT_ELEVATOR_SLOTS {
        let Some(Some(e)) = spec.elevators.get(slot) else {
            w.pad(TDT_ELEVATOR_HEADER_SIZE);
            continue;
        };
        w.u8(1);
        w.u8(e.type_id);
        w.u8([42, 21, 10][e.type_id as usize]);
        w.u8(e.cars);
        w.pad(56);
        w.u8(1);
        w.u8(0);
        w.u16(e.x as u16);
        w.u8(e.top as u8);
        w.u8(e.bottom as u8);
        let serviced: Vec<i64> = e
            .serviced
            .clone()
            .unwrap_or_else(|| (e.bottom..=e.top).collect());
        for fl in 0..TDT_FLOOR_COUNT {
            w.u8(serviced.contains(&fl) as u8);
        }
        for c in 0..8 {
            let home = e
                .car_homes
                .as_ref()
                .and_then(|h| h.get(c).copied())
                .unwrap_or(e.bottom);
            w.u8(home as u8);
        }
        assert!(
            e.top >= e.bottom,
            "an inverted span {}..{}",
            e.bottom,
            e.top
        );
        let written = serviced
            .iter()
            .filter(|&&f| (0..TDT_FLOOR_COUNT).contains(&f) && f >= e.bottom && f <= e.top)
            .count() as i64;
        let express = e.type_id == 0;
        let floors = if let Some(n) = e.payload_floors {
            n
        } else {
            match spec.layout {
                Layout::LegacyServiced => written.max(1),
                Layout::SpannedExpress => e.top - e.bottom + 1,
                Layout::Game if express => express_eligible(e.bottom, e.top).max(1),
                Layout::StoppedExpress if express => written.max(1),
                _ => e.top - e.bottom + 1,
            }
        };
        w.fill(payload_size(floors), fill);
    }

    if spec.generation_one_blocks {
        w.pad(TDT_FINANCE_SIZE);
        w.u16(spec.parking_connected);
        w.pad(TDT_PARKING_SIZE - 2);
    } else {
        w.pad(TDT_STATS_SIZE + TDT_FINANCE_SIZE + TDT_UNKNOWN_FLAGS_SIZE + TDT_UNKNOWN_BYTES_SIZE);
        w.u16(spec.parking_connected);
        w.pad(TDT_PARKING_SIZE - 2);
        w.pad(TDT_CLINIC_LIST_SIZE);
    }
    for slot in 0..TDT_STAIR_SLOTS {
        let Some(s) = spec.stairs.get(slot) else {
            w.pad(TDT_STAIR_RECORD_SIZE);
            continue;
        };
        w.u8(1);
        w.u8(s.type_id);
        w.u16(s.x as u16);
        w.u16(s.floor as u16);
        w.u16(0);
        w.u16(0);
    }
    w.pad(spec.trailing_zeros);
    w.fill(spec.routing_tail, 0xff);
    finish(w.0, spec)
}

fn finish(mut bytes: Vec<u8>, spec: &Spec) -> Vec<u8> {
    if let Some(at) = spec.truncate_at {
        bytes.truncate(at);
    }
    bytes
}

/// A mid-size tower: a ground lobby, offices, condos, a hotel floor with
/// housekeeping (one guest asleep, one room dirty), fast food, a room under
/// construction, parking on B1, an elevator and a stairway.
pub fn sample() -> Spec {
    let t = |left, right, type_id, status| Tenant {
        status,
        ..tenant(left, right, type_id)
    };
    Spec {
        level: 2,
        balance: 15_000,
        current_day: 3,
        floors: vec![
            floor(9, vec![tenant(100, 116, 44), tenant(116, 120, 11)]),
            Floor {
                left_edge: Some(80),
                right_edge: Some(220),
                ..floor(10, vec![tenant(80, 220, 24)])
            },
            floor(
                11,
                vec![
                    t(100, 109, 7, 1),
                    Tenant {
                        rent_rate: Some(3),
                        ..tenant(109, 118, 7)
                    },
                ],
            ),
            floor(
                12,
                vec![t(90, 106, 9, 2), tenant(106, 122, 9), tenant(122, 138, 12)],
            ),
            floor(
                13,
                vec![
                    t(90, 94, 3, 16 | 1),
                    t(94, 98, 3, 32),
                    tenant(98, 104, 4),
                    tenant(104, 112, 15),
                ],
            ),
            floor(14, vec![tenant(100, 109, -7)]),
        ],
        people: 12,
        retail_rows: 1,
        elevators: vec![Some(elevator(1, 2, 150, 9, 14))],
        stairs: vec![Stair {
            type_id: 1,
            x: 130,
            floor: 10,
        }],
        parking_connected: 1,
        ..Spec::default()
    }
}
