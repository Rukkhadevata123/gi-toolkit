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
    find_string: 4993584,
    set_field_of_view: 17468464,
    set_enable_fog_rendering: 280284672,
    set_target_frame_rate: 280206048,
    open_team: 171588976,
    open_team_page_accordingly: 171470064,
    check_can_enter: 209449984,
    craft_entry: 177556768,
    craft_entry_partner: 99470272,
};

pub const ASSETS_PATH: &str = "../assets";
