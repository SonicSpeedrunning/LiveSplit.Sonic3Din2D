#![no_std]
use asr::{signature::Signature, timer, timer::TimerState, watcher::Watcher, Address, Process, time::Duration};

#[cfg(all(not(test), target_arch = "wasm32"))]
#[panic_handler]
fn panic(_: &core::panic::PanicInfo) -> ! {
    core::arch::wasm32::unreachable()
}

static AUTOSPLITTER: spinning_top::Spinlock<State> = spinning_top::const_spinlock(State {
    game: None,
    settings: None,
    watchers: Watchers {
        scene_id: Watcher::new(),
    },
});

struct State {
    game: Option<ProcessInfo>,
    settings: Option<Settings>,
    watchers: Watchers,
}

struct ProcessInfo {
    game: Process,
    main_module_base: Address,
    main_module_size: u64,
    addresses: Option<MemoryPtr>,
}

struct Watchers {
    scene_id: Watcher<Acts>,
}

struct MemoryPtr {
    base_address: Address,
}


#[derive(asr::Settings)]
struct Settings {
    #[default = true]
    /// START: Enable auto start
    start: bool,
    #[default = true]
    /// RESET: Enable auto reset
    reset: bool,
    #[default = true]
    /// Green Grove - Act 1
    green_grove_1: bool,
    #[default = true]
    /// Green Grove - Act 2
    green_grove_2: bool,
    #[default = true]
    /// Rusty Ruin - Act 1
    rusty_ruin_1: bool,
    #[default = true]
    /// Rusty Ruin - Act 2
    rusty_ruin_2: bool,
    #[default = true]
    /// Spring Stadium - Act 1
    spring_stadium_1: bool,
    #[default = true]
    /// Spring Stadium - Act 2
    spring_stadium_2: bool,
    #[default = true]
    /// Diamond Dust - Act 1
    diamond_dust_1: bool,
    #[default = true]
    /// Diamond Dust - Act 2
    diamond_dust_2: bool,
    #[default = true]
    /// Volcano Gallery - Act 1
    volcano_gallery_1: bool,
    #[default = true]
    /// Volcano Gallery - Act 2
    volcano_gallery_2: bool,
    #[default = true]
    /// Gene Gadget - Act 1
    gene_gadget_1: bool,
    #[default = true]
    /// Gene Gadget - Act 2
    gene_gadget_2: bool,
    #[default = true]
    /// Panic Puppet - Act 1
    panic_puppet_1: bool,
    #[default = true]
    /// Panic Puppet - Act 2
    panic_puppet_2: bool,
    #[default = true]
    /// Final Fight
    final_fight: bool,
}

impl ProcessInfo {
    fn attach_process() -> Option<Self> {
        const PROCESS_NAMES: [&str; 17] =
            ["Sonic3D2d 1.26rc.exe", "Sonic3D2d 1.26.exe", "Sonic3D2d 1.26b.exe", "Sonic3D2d 1.27.exe", "Sonic3D2d 1.28.exe",
            "Sonic3D2d 1.29.exe", "Sonic3D2d 1.30.exe", "Sonic3D2d 1.31.exe", "Sonic3D2d 1.32.exe", "Sonic3D2d 1.33.exe",
            "Sonic3D2d 1.34.exe", "Sonic3D2d 1.35.exe", "Sonic3D2d 1.36.exe", "Sonic3D2d 1.37.exe", "Sonic3D2d 1.38.exe",
            "Sonic3D2d 1.39.exe", "Sonic3D2d 1.40.exe"];
        let mut proc: Option<Process> = None;
        let mut proc_name: Option<&str> = None;
    
        for name in PROCESS_NAMES {
            proc = Process::attach(name);
            if proc.is_some() {
                proc_name = Some(name);
                break
            }
        }
    
        let game = proc?;
        let main_module_base = game.get_module_address(proc_name?).ok()?;
        let main_module_size = game.get_module_size(proc_name?).ok()?;

        Some(Self {
            game,
            main_module_base,
            main_module_size,
            addresses: None,
        })
    }

    fn look_for_addresses(&mut self) -> Option<MemoryPtr> {
        const SIG: Signature<9> = Signature::new("8B 3D ???????? 8B 0C 87");

        let game = &self.game;
        let ptr = SIG.scan_process_range(game, self.main_module_base, self.main_module_size)?.0 + 2;
        let base_address = Address(game.read::<u32>(Address(ptr)).ok()? as u64);

        Some(MemoryPtr {
            base_address,
        })
    }
}

impl State {
    fn init(&mut self) -> bool {        
        if self.game.is_none() {
            self.game = ProcessInfo::attach_process()
        }

        let Some(game) = &mut self.game else {
            return false
        };

        if !game.game.is_open() {
            self.game = None;
            return false
        }

        if game.addresses.is_none() {
            game.addresses = game.look_for_addresses()
        }

        game.addresses.is_some()   
    }

    fn update(&mut self) {
        let Some(game) = &self.game else { return };
        let Some(addresses) = &game.addresses else { return };
        let proc = &game.game;

        let sceneid = proc.read_pointer_path32::<u16>(addresses.base_address.0 as u32, &[0, 0x0, 0x268, 0x2C8]).ok().unwrap_or_default();
        self.watchers.scene_id.update(Some(match sceneid {
            1 => Acts::GreenGroveAct1,
            2 => Acts::GreenGroveAct2,
            3 => Acts::RustyRuinAct1,
            4 => Acts::RustyRuinAct2,
            5 => Acts::SpringStadiumAct1,
            6 => Acts::SpringStadiumAct2,
            7 => Acts::DiamondDustAct1,
            8 => Acts::DiamondDustAct2,
            9 => Acts::VolcanoGalleryAct1,
            10 => Acts::VolcanoGalleryAct2,
            11 => Acts::GeneGadgetAct1,
            12 => Acts::GeneGadgetAct2,
            13 => Acts::PanicPuppetAct1,
            14 => Acts::PanicPuppetAct2,
            15 => Acts::FinalFight,
            17 => Acts::GameStart,
            18 => Acts::Ending,
            19 => Acts::NewGameMenu,
            20 => Acts::ContinueMenu,
            _ => match &self.watchers.scene_id.pair {
                Some(thing) => thing.current,
                _ => Acts::Undefined,
                },
        }));
    }

    fn start(&mut self) -> bool {
        let Some(settings) = &self.settings else { return false };
        if !settings.start { return false };

        let Some(level_id) = &self.watchers.scene_id.pair else { return false };
        level_id.old == Acts::NewGameMenu && level_id.current == Acts::GameStart    
    }

    fn split(&mut self) -> bool {
        let Some(settings) = &self.settings else { return false };
        let Some(level_id) = &self.watchers.scene_id.pair else { return false };
        
        match level_id.old {
            Acts::GreenGroveAct1 => settings.green_grove_1 && level_id.current == Acts::GreenGroveAct2,
            Acts::GreenGroveAct2 => settings.green_grove_2 && level_id.current == Acts::RustyRuinAct1,
            Acts::RustyRuinAct1 => settings.rusty_ruin_1 && level_id.current == Acts::RustyRuinAct2,
            Acts::RustyRuinAct2 => settings.rusty_ruin_2 && level_id.current == Acts::SpringStadiumAct1,
            Acts::SpringStadiumAct1 => settings.spring_stadium_1 && level_id.current == Acts::SpringStadiumAct2,
            Acts::SpringStadiumAct2 => settings.spring_stadium_2 && level_id.current == Acts::DiamondDustAct1,
            Acts::DiamondDustAct1 => settings.diamond_dust_1 && level_id.current == Acts::DiamondDustAct2,
            Acts::DiamondDustAct2 => settings.diamond_dust_2 && level_id.current == Acts::VolcanoGalleryAct1,
            Acts::VolcanoGalleryAct1 => settings.volcano_gallery_1 && level_id.current == Acts::VolcanoGalleryAct2,
            Acts::VolcanoGalleryAct2 => settings.volcano_gallery_2 && level_id.current == Acts::GeneGadgetAct1,
            Acts::GeneGadgetAct1 => settings.gene_gadget_1 && level_id.current == Acts::GeneGadgetAct2,
            Acts::GeneGadgetAct2 => settings.gene_gadget_2 && level_id.current == Acts::PanicPuppetAct1,
            Acts::PanicPuppetAct1 => settings.panic_puppet_1 && level_id.current == Acts::PanicPuppetAct2,
            Acts::PanicPuppetAct2 => settings.panic_puppet_2 && (level_id.current == Acts::FinalFight || level_id.current == Acts::Ending),
            Acts::FinalFight => settings.final_fight && level_id.current == Acts::Ending,
            _ => false
        }
    }

    fn reset(&mut self) -> bool {
        let Some(settings) = &self.settings else { return false };
        if !settings.reset { return false };

        let Some(level_id) = &self.watchers.scene_id.pair else { return false };
        level_id.current != level_id.old && level_id.current == Acts::NewGameMenu
    }

    fn is_loading(&mut self) -> Option<bool> {
        None
    }

    fn game_time(&mut self) -> Option<Duration> {
        None
    }
}

#[no_mangle]
pub extern "C" fn update() {
    // Get access to the spinlock
    let autosplitter = &mut AUTOSPLITTER.lock();
    
    // Sets up the settings
    autosplitter.settings.get_or_insert_with(Settings::register);

    // Main autosplitter logic, essentially refactored from the OG LivaSplit autosplitting component.
    // First of all, the autosplitter needs to check if we managed to attach to the target process,
    // otherwise there's no need to proceed further.
    if !autosplitter.init() {
        return
    }

    // The main update logic is launched with this
    autosplitter.update();

    // Splitting logic. Adapted from OG LiveSplit:
    // Order of execution
    // 1. update() [this is launched above] will always be run first. There are no conditions on the execution of this action.
    // 2. If the timer is currently either running or paused, then the isLoading, gameTime, and reset actions will be run.
    // 3. If reset does not return true, then the split action will be run.
    // 4. If the timer is currently not running (and not paused), then the start action will be run.
    if timer::state() == TimerState::Running || timer::state() == TimerState::Paused {
        if let Some(is_loading) = autosplitter.is_loading() {
            if is_loading {
                timer::pause_game_time()
            } else {
                timer::resume_game_time()
            }
        }

        if let Some(game_time) = autosplitter.game_time() {
            timer::set_game_time(game_time)
        }

        if autosplitter.reset() {
            timer::reset()
        } else if autosplitter.split() {
            timer::split()
        }
    } 

    if timer::state() == TimerState::NotRunning {
        if autosplitter.start() {
            timer::start();

            if let Some(is_loading) = autosplitter.is_loading() {
                if is_loading {
                    timer::pause_game_time()
                } else {
                    timer::resume_game_time()
                }
            }
        }
    }     
}

#[derive(Clone, Copy, PartialEq)]
enum Acts {
    GreenGroveAct1,
    GreenGroveAct2,
    RustyRuinAct1,
    RustyRuinAct2,
    SpringStadiumAct1,
    SpringStadiumAct2,
    DiamondDustAct1,
    DiamondDustAct2,
    VolcanoGalleryAct1,
    VolcanoGalleryAct2,
    GeneGadgetAct1,
    GeneGadgetAct2,
    PanicPuppetAct1,
    PanicPuppetAct2,
    FinalFight,
    GameStart,
    Ending,
    NewGameMenu,
    ContinueMenu,
    Undefined,
}