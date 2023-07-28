use ctor::{ctor, dtor};
use env_logger::{self, Env, Target};
use libmem::*;
use log::info;
use once_cell::sync::OnceCell;
use std::env;
use std::ffi::{CStr, CString};
use std::fs::File;
use std::io::Write;
use std::mem::{self, MaybeUninit};

macro_rules! ptr_offset {
    ($ptr:expr, $offset:expr) => {
        ($ptr as usize + $offset as usize) as usize
    };
}

macro_rules! field_func {
    ($name:ident, $type:ty, $offset:literal) => {
        unsafe fn $name(&self) -> *mut $type {
            ptr_offset!(*self.base, $offset) as *mut $type
        }
    };
}

#[derive(Debug)]
struct PlayerEnt {
    base: *const *const (),
}

impl PlayerEnt {
    fn new(base: *const *const ()) -> Self {
        Self { base }
    }

    field_func!(health, i32, 256);
}

fn setup_logger() {
    let filename = "rustycube.log";
    let log_path = match env::var("HOME") {
        Ok(dir) => format!("{}/{}", dir, filename),
        Err(_) => format!("{}", filename),
    };
    let log_file = Box::new(File::create(log_path).expect("[RC] Failed to create log file"));

    // TODO: Use 'debug' if debug build, 'info' if release build
    env_logger::builder()
        .parse_env(Env::new().default_filter_or("debug"))
        .target(Target::Pipe(log_file))
        .format(|buf, record| writeln!(buf, "[RC] <{}> {}", record.level(), record.args()))
        .init();
}

static mut gl_drawframe_tramp: (usize, usize) = (0, 0);

#[allow(non_camel_case_types)]
extern "C" fn hk_gl_drawframe(w: i32, h: i32, changelod: f32, curfps: f32, elapsed: i32) {
    type gl_drawframe_fn = extern "C" fn(i32, i32, f32, f32, i32);
    info!("Screen: ({}, {})", w, h);
    info!("FPS: {}", curfps);
    let orig = unsafe { mem::transmute::<usize, gl_drawframe_fn>(gl_drawframe_tramp.0) };
    return orig(w, h, changelod, curfps, elapsed);
}

static mut menu_name: OnceCell<CString> = OnceCell::new();
static mut menu_title: OnceCell<CString> = OnceCell::new();

static mut menuitemmanual_addr: usize = 0;
static mut menureset_addr: usize = 0;
static menu_content: OnceCell<CString> = OnceCell::new();
static menu_action: OnceCell<CString> = OnceCell::new();
static menuitem_hello: OnceCell<CString> = OnceCell::new();
static menuitem_hello_action: OnceCell<CString> = OnceCell::new();

extern "C" fn refreshmenu(menu: *const (), init: bool) {
    type menuitemmanual_fn = extern "C" fn(*const (), *const i8, *const i8, *const (), *const i8);
    type menureset_fn = extern "C" fn(*const ());
    let menuitemmanual = unsafe { mem::transmute::<usize, menuitemmanual_fn>(menuitemmanual_addr) };
    let menureset = unsafe { mem::transmute::<usize, menureset_fn>(menureset_addr) };

    menureset(menu);

    let content = menuitem_hello.get_or_init(|| CString::new("Hello").unwrap());
    let action = menuitem_hello_action.get_or_init(|| CString::new("rustysay hello").unwrap());

    menuitemmanual(
        menu,
        content.as_ptr(),
        action.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
    );

    let content = menu_content.get_or_init(|| CString::new("Close").unwrap());
    let action = menu_action.get_or_init(|| CString::new("closecurmenu").unwrap());

    menuitemmanual(
        menu,
        content.as_ptr(),
        action.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
    );
}

static mut command_menu: OnceCell<CString> = OnceCell::new();
static mut command_sig: OnceCell<CString> = OnceCell::new();
static mut curmenu: *mut *const () = 0 as *mut *const ();
static mut mymenu: *const () = 0 as *const ();
static mut command_mysay: OnceCell<CString> = OnceCell::new();
static mut command_mysay_sig: OnceCell<CString> = OnceCell::new();

extern "C" fn cmdrustymenu() {
    unsafe { *curmenu = mymenu };
}

static mut rustysay_str: MaybeUninit<CString> = MaybeUninit::uninit();
static mut rustysay_action: OnceCell<CString> = OnceCell::new();
static mut menu_rustysay: *const () = 0 as *const ();
static mut menu_rustysay_name: OnceCell<CString> = OnceCell::new();
static mut menu_rustysay_title: OnceCell<CString> = OnceCell::new();

extern "C" fn cmdrustysay(text: *const i8) {
    info!("RustySay: {:?}", text);
    let cstr = unsafe { CStr::from_ptr(text) };
    unsafe { rustysay_str.write(cstr.to_owned()) };
    unsafe { *curmenu = menu_rustysay };
}

extern "C" fn refreshrustysay(menu: *const (), init: bool) {
    type menuitemmanual_fn = extern "C" fn(*const (), *const i8, *const i8, *const (), *const i8);
    type menureset_fn = extern "C" fn(*const ());
    let menuitemmanual = unsafe { mem::transmute::<usize, menuitemmanual_fn>(menuitemmanual_addr) };
    let menureset = unsafe { mem::transmute::<usize, menureset_fn>(menureset_addr) };

    menureset(menu);

    let content = unsafe { rustysay_str.assume_init_ref() };
    let action = unsafe { rustysay_action.get_or_init(|| CString::new("closecurmenu").unwrap()) };

    menuitemmanual(
        menu,
        content.as_ptr(),
        action.as_ptr(),
        std::ptr::null(),
        std::ptr::null(),
    );
}

#[ctor]
unsafe fn lib_entry() {
    setup_logger();
    info!("Loaded");

    let ac_client = LM_FindModule("linux_64_client").unwrap();
    info!("Client Module: {}", ac_client);

    let player1 =
        PlayerEnt::new(LM_FindSymbolAddress(&ac_client, "player1").unwrap() as *const *const ());
    info!("Local Player Address: {:?}", player1);
    info!("Health Address: {:x?}", player1.health());
    info!("Local Player Health: {}", *(player1.health()));
    *(player1.health()) = 1000;
    info!("Set Player Health to 1000");

    type addmenu_fn = extern "C" fn(usize, usize, bool, usize, usize, bool, bool) -> *const ();
    let addmenu = unsafe {
        mem::transmute::<usize, addmenu_fn>(
            LM_FindSymbolAddress(&ac_client, "_Z7addmenuPKcS0_bPFvPvbEPFbS1_ibEbb").unwrap(),
        )
    };
    info!("addmenu Address: {:?}", addmenu);

    menu_name.set(CString::new("rustycube").unwrap()).unwrap();
    menu_title.set(CString::new("Rusty Cube").unwrap()).unwrap();
    mymenu = addmenu(
        menu_name.get().unwrap().as_ptr() as usize,
        menu_title.get().unwrap().as_ptr() as usize,
        true,
        refreshmenu as *const () as usize,
        0,
        false,
        false,
    );

    menu_rustysay_name
        .set(CString::new("rustysay").unwrap())
        .unwrap();
    menu_rustysay_title
        .set(CString::new("RustySay").unwrap())
        .unwrap();
    menu_rustysay = addmenu(
        menu_rustysay_name.get().unwrap().as_ptr() as usize,
        menu_rustysay_title.get().unwrap().as_ptr() as usize,
        true,
        refreshrustysay as *const () as usize,
        0,
        false,
        false,
    );

    curmenu = LM_FindSymbolAddress(&ac_client, "curmenu").unwrap() as *mut *const ();
    info!("curmenu Address: {:?}", curmenu);
    *curmenu = mymenu;

    menuitemmanual_addr =
        LM_FindSymbolAddress(&ac_client, "_Z14menuitemmanualPvPcS0_P5colorPKc").unwrap();

    menureset_addr = LM_FindSymbolAddress(&ac_client, "_Z9menuresetPv").unwrap();

    type addcommand_fn = extern "C" fn(*const i8, *const (), *const i8) -> bool;
    let addcommand_addr = LM_FindSymbolAddress(&ac_client, "_Z10addcommandPKcPFvvES0_").unwrap();
    let addcommand = unsafe { mem::transmute::<usize, addcommand_fn>(addcommand_addr) };
    command_menu
        .set(CString::new("rustycube").unwrap())
        .unwrap();
    command_sig.set(CString::new("").unwrap()).unwrap();
    addcommand(
        command_menu.get().unwrap().as_ptr(),
        cmdrustymenu as *const (),
        command_sig.get().unwrap().as_ptr(),
    );

    command_mysay
        .set(CString::new("rustysay").unwrap())
        .unwrap();
    command_mysay_sig.set(CString::new("s").unwrap()).unwrap();
    addcommand(
        command_mysay.get().unwrap().as_ptr(),
        cmdrustysay as *const (),
        command_mysay_sig.get().unwrap().as_ptr(),
    );

    let sdl_window = LM_FindSymbolAddress(&ac_client, "screen").unwrap();
    info!("SDL Window Handle: {:#x}", sdl_window);

    let gl_drawframe = LM_FindSymbolAddress(&ac_client, "_Z12gl_drawframeiiffi").unwrap();
    info!("gl_drawframe Address: {:#x}", gl_drawframe);

    LM_ProtMemory(gl_drawframe as usize, 0x1024, LM_PROT_XRW);
    gl_drawframe_tramp = LM_HookCode(gl_drawframe, hk_gl_drawframe as usize).unwrap();
}

#[dtor]
fn lib_exit() {
    info!("Unloaded")
}
