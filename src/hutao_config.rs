#[repr(C)]
#[derive(Debug, Clone, Copy)]
pub struct FunctionOffsets {
    pub find_string: u32,
    pub set_field_of_view: u32,
    pub set_enable_fog_rendering: u32,
    pub set_target_frame_rate: u32,
    pub open_team: u32,
    pub open_team_page_accordingly: u32,
    pub check_can_enter: u32,
    pub craft_entry: u32,
    pub craft_entry_partner: u32,
}

#[repr(i32)]
#[derive(Debug, Clone, Copy, PartialEq)]
pub enum IslandState {
    None = 0,
    Error = 1,
    Started = 2,
    Stopped = 3,
}

#[repr(C)]
#[derive(Debug)]
pub struct IslandEnvironment {
    pub state: IslandState,
    pub last_error: u32,
    pub function_offsets: FunctionOffsets,
    pub field_of_view: f32,
    pub fix_low_fov_scene: i32, // BOOL
    pub disable_fog: i32,       // BOOL
    pub target_frame_rate: i32,
    pub remove_open_team_progress: i32, // BOOL
    pub redirect_craft_entry: i32,      // BOOL
}

pub const SHARED_MEMORY_NAME: &str = "4F3E8543-40F7-4808-82DC-21E48A6037A7";
pub const CHINESE_OFFSETS: FunctionOffsets = FunctionOffsets {
    find_string: 4830752,
    set_field_of_view: 17204528,
    set_enable_fog_rendering: 277807600,
    set_target_frame_rate: 277729120,
    open_team: 118414576,
    open_team_page_accordingly: 118384496,
    check_can_enter: 156982512,
    craft_entry: 127845632,
    craft_entry_partner: 201143472,
};
