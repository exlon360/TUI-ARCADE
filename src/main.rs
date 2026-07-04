use std::collections::{HashMap, HashSet, VecDeque};
use std::env;
use std::fs;
use std::io::{self, Read, Write};
use std::path::PathBuf;
use std::process::{Child, Command, Stdio};
use std::sync::{Mutex, OnceLock};
use std::thread;
use std::time::{Duration, Instant, SystemTime, UNIX_EPOCH};

const TITLE_ART: [&str; 5] = [
    " _______  _     _  _____      ___    ____   ____    _    ____  _____ ",
    "|__   __|| |   | ||_   _|    / _ \\  |  _ \\ / ___|  / \\  |  _ \\| ____|",
    "   | |   | |   | |  | |     | |_| | | |_) | |     / _ \\ | | | |  _|  ",
    "   | |   | |___| |  | |     |  _  | |  _ <| |___ / ___ \\| |_| | |___ ",
    "   |_|    \\_____/   |_|     |_| |_| |_| \\_\\\\____/_/   \\_\\____/|_____|",
];

const DEFAULT_TITLE: &str = "TUI Arcade";
const MAX_TITLE_LEN: usize = 60;
const SAVED_THEME_SLOTS: usize = 3;

const COLOR_NAMES: [&str; 16] = [
    "Black", "Red", "Green", "Yellow", "Blue", "Magenta", "Cyan", "White", "Gray", "Hot Red",
    "Lime", "Gold", "Sky", "Pink", "Aqua", "Bright",
];

#[derive(Clone)]
struct Difficulty {
    name: &'static str,
    description: &'static str,
    speed: f64,
    lives: i32,
    tick_ms: u64,
}

const DIFFICULTIES: [Difficulty; 4] = [
    Difficulty {
        name: "Easy",
        description: "Forgiving timing, extra lives, slower enemies.",
        speed: 0.75,
        lives: 5,
        tick_ms: 95,
    },
    Difficulty {
        name: "Normal",
        description: "Classic terminal arcade pace.",
        speed: 1.0,
        lives: 4,
        tick_ms: 75,
    },
    Difficulty {
        name: "Hard",
        description: "Faster, tighter, and less forgiving.",
        speed: 1.28,
        lives: 3,
        tick_ms: 55,
    },
    Difficulty {
        name: "Master",
        description: "Brutal speed, scarce lives, and almost no recovery room.",
        speed: 1.55,
        lives: 2,
        tick_ms: 42,
    },
];

const PONG_ASSIST_NAMES: [&str; 3] = ["Off", "Light", "Strong"];
const PONG_SPEED_NAMES: [&str; 3] = ["Calm", "Classic", "Fast"];
const PONG_SPEED_FACTORS: [f64; 3] = [0.82, 1.0, 1.14];

#[derive(Clone)]
struct Theme {
    name: String,
    fg: u8,
    bg: Option<u8>,
    title: u8,
    accent: u8,
    secondary: u8,
    danger: u8,
    success: u8,
    muted: u8,
    highlight: u8,
}

#[derive(Clone)]
struct GlyphSet {
    name: &'static str,
    description: &'static str,
    top_left: &'static str,
    top_right: &'static str,
    bottom_left: &'static str,
    bottom_right: &'static str,
    horizontal: &'static str,
    vertical: &'static str,
    title_left: &'static str,
    title_right: &'static str,
    selector: &'static str,
    scroll_up: &'static str,
    scroll_down: &'static str,
    scroll_track: &'static str,
    scroll_thumb: &'static str,
    button_left: &'static str,
    button_right: &'static str,
}

#[derive(Clone, Copy)]
enum Role {
    Normal,
    Title,
    Accent,
    Secondary,
    Danger,
    Success,
    Muted,
    Highlight,
}

#[derive(Clone, Copy)]
struct Controls {
    up: char,
    down: char,
    left: char,
    right: char,
    action: char,
    pause: char,
    quit: char,
}

impl Default for Controls {
    fn default() -> Self {
        Self {
            up: 'w',
            down: 's',
            left: 'a',
            right: 'd',
            action: ' ',
            pause: 'p',
            quit: 'q',
        }
    }
}

static ACTIVE_CONTROLS: OnceLock<Mutex<Controls>> = OnceLock::new();

#[derive(Clone, Copy, PartialEq, Eq)]
enum Key {
    Up,
    Down,
    Left,
    Right,
    Enter,
    Esc,
    Space,
    Backspace,
    Char(char),
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum GameCategory {
    All,
    Favorites,
    Arcade,
    Action,
    Puzzle,
    Strategy,
    Reflex,
    Racing,
    Adventure,
}

const CATEGORIES: [GameCategory; 9] = [
    GameCategory::All,
    GameCategory::Favorites,
    GameCategory::Arcade,
    GameCategory::Action,
    GameCategory::Puzzle,
    GameCategory::Strategy,
    GameCategory::Reflex,
    GameCategory::Racing,
    GameCategory::Adventure,
];

impl GameCategory {
    fn name(self) -> &'static str {
        match self {
            GameCategory::All => "All",
            GameCategory::Favorites => "Favorites",
            GameCategory::Arcade => "Arcade",
            GameCategory::Action => "Action",
            GameCategory::Puzzle => "Puzzle",
            GameCategory::Strategy => "Strategy",
            GameCategory::Reflex => "Reflex",
            GameCategory::Racing => "Racing",
            GameCategory::Adventure => "Adventure",
        }
    }
}

#[derive(Clone, Copy)]
enum GameKind {
    Snake,
    Tetris,
    Pong,
    TronCycles,
    TronGridRun,
    Invaders,
    Missile,
    Breakout,
    Meteor,
    Racer,
    Frog,
    Target,
    Coin,
    Minefield,
    Maze,
    Whack,
    Simon,
    Reaction,
    Flappy,
    Asteroid,
    Star,
    Laser,
    Dungeon,
    River,
    Memory,
    Number,
    Circuit,
    Orbit,
    BlockDrop,
    CometCatcher,
    BombSweeper,
    NeonDrift,
    CargoCatch,
    GemRush,
    TrapRunner,
    ReactorTrace,
    DroneDodge,
    PearlDiver,
    SolarSailer,
    VaultEscape,
    DataStorm,
    PixelPop,
    BugHunt,
    FuelRun,
    SparkChase,
    IceSlide,
    SignalTrace,
    OrbitalCourier,
    RainRunner,
    ByteBlaster,
    StormSurge,
    CrystalCavern,
    TicTacToe,
    Chess,
    Checkers,
    Micro(usize),
}

struct GameInfo {
    name: &'static str,
    summary: &'static str,
    kind: GameKind,
}

#[derive(Clone, Copy)]
enum QuestKind {
    Checkmate,
    Cipher,
    Marble,
    Quantum,
    Go,
    Pirate,
    Samurai,
    Mars,
    DeepSea,
    Volcano,
    Jungle,
    Dragon,
    Mirror,
}

#[derive(Clone, Copy)]
enum LaneKind {
    Rune,
    Sea,
    AirHockey,
    Hockey,
    Ski,
    Snowboard,
    Bmx,
    Horse,
    Ninja,
    Moon,
    Saturn,
    Submarine,
    Desert,
    Time,
}

#[derive(Clone, Copy)]
enum CatchKind {
    Glyph,
    Poker,
    Pinball,
    Tennis,
    Cricket,
    Alien,
    Astro,
    Castle,
    Potion,
}

#[derive(Clone, Copy)]
enum AimKind {
    Basket,
    Archery,
    Curling,
}

#[derive(Clone, Copy)]
enum SequenceKind {
    Factory,
    Duel,
    Trick,
}

#[derive(Clone, Copy)]
enum WordKind {
    Vault,
    Hangman,
}

#[derive(Clone, Copy)]
enum MicroMode {
    ConnectFour,
    WordGuess(WordKind),
    Blackjack,
    BlackjackBlitz,
    Battleship,
    TowerStack,
    LightsOut,
    SlidePuzzle,
    DominoChain,
    MiniGolf,
    Darts,
    Mancala,
    MiniSudoku,
    Reversi,
    Bowling,
    SkeeBall,
    Keeper,
    Quest(QuestKind),
    Lane(LaneKind),
    Catch(CatchKind),
    Aim(AimKind),
    Sequence(SequenceKind),
}

#[derive(Clone, Copy)]
struct MicroGame {
    category: GameCategory,
    mode: MicroMode,
}

const GAMES: &[GameInfo] = &[
    GameInfo {
        name: "Space Invaders",
        summary: "Dodge bombs, use shields, and clear the alien block.",
        kind: GameKind::Invaders,
    },
    GameInfo {
        name: "Tetris",
        summary: "Stack falling pieces, clear rows, and keep the well open.",
        kind: GameKind::Tetris,
    },
    GameInfo {
        name: "Snake",
        summary: "Eat food, grow longer, and avoid walls and your own tail.",
        kind: GameKind::Snake,
    },
    GameInfo {
        name: "Missile Command",
        summary: "Aim the reticle and blast incoming missiles before they hit cities.",
        kind: GameKind::Missile,
    },
    GameInfo {
        name: "Pong",
        summary: "Fast paddle duel with wall, paddle, and score sounds.",
        kind: GameKind::Pong,
    },
    GameInfo {
        name: "Tron Light Cycles",
        summary: "Out-turn a CPU rider while both hard-light trails become walls.",
        kind: GameKind::TronCycles,
    },
    GameInfo {
        name: "Tron Grid Run",
        summary: "Steer a one-way light trail, collect cores, and avoid your own path.",
        kind: GameKind::TronGridRun,
    },
    GameInfo {
        name: "Breakout",
        summary: "Bounce the ball, clear bricks, and keep it above your paddle.",
        kind: GameKind::Breakout,
    },
    GameInfo {
        name: "Meteor Dodge",
        summary: "Slide through falling rocks and survive the storm.",
        kind: GameKind::Meteor,
    },
    GameInfo {
        name: "Racer",
        summary: "Thread your car through traffic lanes.",
        kind: GameKind::Racer,
    },
    GameInfo {
        name: "Frog Cross",
        summary: "Hop through moving traffic to reach the far bank.",
        kind: GameKind::Frog,
    },
    GameInfo {
        name: "Target Practice",
        summary: "Move the reticle and tag targets before time runs out.",
        kind: GameKind::Target,
    },
    GameInfo {
        name: "Coin Collector",
        summary: "Grab coins, dodge traps, and build a score streak.",
        kind: GameKind::Coin,
    },
    GameInfo {
        name: "Minefield",
        summary: "Find the exit while avoiding hidden mines.",
        kind: GameKind::Minefield,
    },
    GameInfo {
        name: "Maze Runner",
        summary: "Navigate a fresh terminal maze to the goal.",
        kind: GameKind::Maze,
    },
    GameInfo {
        name: "Whack-a-Mole",
        summary: "Move the mallet cursor and hit pop-up targets.",
        kind: GameKind::Whack,
    },
    GameInfo {
        name: "Simon Says",
        summary: "Watch the command sequence and repeat it from memory.",
        kind: GameKind::Simon,
    },
    GameInfo {
        name: "Reaction Test",
        summary: "Hit the prompted key as fast as you can.",
        kind: GameKind::Reaction,
    },
    GameInfo {
        name: "Flappy Dash",
        summary: "Move up and down through scrolling gates. No flap gravity.",
        kind: GameKind::Flappy,
    },
    GameInfo {
        name: "Asteroid Belt",
        summary: "Pilot through side-scrolling asteroid fields.",
        kind: GameKind::Asteroid,
    },
    GameInfo {
        name: "Star Catcher",
        summary: "Catch falling stars while dodging bombs.",
        kind: GameKind::Star,
    },
    GameInfo {
        name: "Laser Drill",
        summary: "Drill blocks with laser shots before they reach you.",
        kind: GameKind::Laser,
    },
    GameInfo {
        name: "Dungeon Crawl",
        summary: "Find treasure, dodge wandering enemies, and reach the stairs.",
        kind: GameKind::Dungeon,
    },
    GameInfo {
        name: "River Raid",
        summary: "Steer down a river, dodge rocks, and collect fuel.",
        kind: GameKind::River,
    },
    GameInfo {
        name: "Memory Match",
        summary: "Flip terminal tiles and match all pairs.",
        kind: GameKind::Memory,
    },
    GameInfo {
        name: "Number Crunch",
        summary: "Solve quick math prompts under pressure.",
        kind: GameKind::Number,
    },
    GameInfo {
        name: "Circuit Trace",
        summary: "Trace nodes in order without crossing live walls.",
        kind: GameKind::Circuit,
    },
    GameInfo {
        name: "Orbit Guard",
        summary: "Rotate around the core and block incoming sparks.",
        kind: GameKind::Orbit,
    },
    GameInfo {
        name: "Block Drop",
        summary: "Catch falling cargo and avoid cracked blocks.",
        kind: GameKind::BlockDrop,
    },
    GameInfo {
        name: "Comet Catcher",
        summary: "Build a comet combo while avoiding hot debris.",
        kind: GameKind::CometCatcher,
    },
    GameInfo {
        name: "Bomb Sweeper",
        summary: "Use the scanner to cross a hidden explosive field.",
        kind: GameKind::BombSweeper,
    },
    GameInfo {
        name: "Neon Drift",
        summary: "Ride drift momentum and keep heat under control.",
        kind: GameKind::NeonDrift,
    },
    GameInfo {
        name: "Cargo Catch",
        summary: "Load the cargo quota while dodging cracked crates.",
        kind: GameKind::CargoCatch,
    },
    GameInfo {
        name: "Gem Rush",
        summary: "Chain gem catches into a bigger combo score.",
        kind: GameKind::GemRush,
    },
    GameInfo {
        name: "Trap Runner",
        summary: "Race through a grid of moving visible traps.",
        kind: GameKind::TrapRunner,
    },
    GameInfo {
        name: "Reactor Trace",
        summary: "Touch reactor nodes in order, then escape.",
        kind: GameKind::ReactorTrace,
    },
    GameInfo {
        name: "Drone Dodge",
        summary: "Pilot a drone through incoming hazard clouds.",
        kind: GameKind::DroneDodge,
    },
    GameInfo {
        name: "Pearl Diver",
        summary: "Collect pearls to refill oxygen under pressure.",
        kind: GameKind::PearlDiver,
    },
    GameInfo {
        name: "Solar Sailer",
        summary: "Build solar charge to slow fuel drain and score.",
        kind: GameKind::SolarSailer,
    },
    GameInfo {
        name: "Vault Escape",
        summary: "Collect every vault key before taking the exit.",
        kind: GameKind::VaultEscape,
    },
    GameInfo {
        name: "Data Storm",
        summary: "Catch data packets in streaks and avoid errors.",
        kind: GameKind::DataStorm,
    },
    GameInfo {
        name: "Pixel Pop",
        summary: "Pop connected color clusters before time runs out.",
        kind: GameKind::PixelPop,
    },
    GameInfo {
        name: "Bug Hunt",
        summary: "Shoot crawling bugs before the swarm grows too big.",
        kind: GameKind::BugHunt,
    },
    GameInfo {
        name: "Fuel Run",
        summary: "Fly a dangerous route and keep fuel topped up.",
        kind: GameKind::FuelRun,
    },
    GameInfo {
        name: "Spark Chase",
        summary: "Weave through sparks and collect glowing orbs.",
        kind: GameKind::SparkChase,
    },
    GameInfo {
        name: "Ice Slide",
        summary: "Slide until blocked through an icy maze.",
        kind: GameKind::IceSlide,
    },
    GameInfo {
        name: "Signal Trace",
        summary: "Trace signal nodes in sequence without getting lost.",
        kind: GameKind::SignalTrace,
    },
    GameInfo {
        name: "Orbital Courier",
        summary: "Hit the packet delivery quota through debris.",
        kind: GameKind::OrbitalCourier,
    },
    GameInfo {
        name: "Rain Runner",
        summary: "Catch rain drops in combos and dodge the bad ones.",
        kind: GameKind::RainRunner,
    },
    GameInfo {
        name: "Byte Blaster",
        summary: "Type falling byte letters before they hit bottom.",
        kind: GameKind::ByteBlaster,
    },
    GameInfo {
        name: "Storm Surge",
        summary: "Fight a shifting current while grabbing fuel.",
        kind: GameKind::StormSurge,
    },
    GameInfo {
        name: "Crystal Cavern",
        summary: "Collect every crystal before escaping the cavern.",
        kind: GameKind::CrystalCavern,
    },
    GameInfo {
        name: "Tic Tac Toe",
        summary: "Classic 3x3 X/O duel against a blocking CPU.",
        kind: GameKind::TicTacToe,
    },
    GameInfo {
        name: "Chess",
        summary: "Play white in a real chess board with legal piece movement and a CPU reply.",
        kind: GameKind::Chess,
    },
    GameInfo {
        name: "Checkers",
        summary: "Jump, crown kings, and clear the board in a CPU checkers duel.",
        kind: GameKind::Checkers,
    },
    GameInfo {
        name: "Connect Four",
        summary: "Drop X pieces into columns and race the CPU to four in a row.",
        kind: GameKind::Micro(0),
    },
    GameInfo {
        name: "Checkmate Dash",
        summary: "Use knight-style leaps through a maze and land on the king.",
        kind: GameKind::Micro(1),
    },
    GameInfo {
        name: "Word Vault",
        summary: "Guess the hidden terminal word before the vault locks.",
        kind: GameKind::Micro(2),
    },
    GameInfo {
        name: "Glyph Garden",
        summary: "Catch rare glyph blooms while avoiding thorn marks.",
        kind: GameKind::Micro(3),
    },
    GameInfo {
        name: "Rune Runner",
        summary: "Swap lanes, jump rune traps, and collect enough sigils.",
        kind: GameKind::Micro(4),
    },
    GameInfo {
        name: "Cipher Chase",
        summary: "Trace cipher nodes in sequence before the grid beats you.",
        kind: GameKind::Micro(5),
    },
    GameInfo {
        name: "Hangman Vault",
        summary: "Pick letters, reveal the word, and survive limited wrong guesses.",
        kind: GameKind::Micro(6),
    },
    GameInfo {
        name: "Mancala Rush",
        summary: "Sow stones around pits, capture opposite gems, and beat the CPU store.",
        kind: GameKind::Micro(7),
    },
    GameInfo {
        name: "Sudoku Sweep",
        summary: "Fill a 4x4 number grid without row, column, or box repeats.",
        kind: GameKind::Micro(8),
    },
    GameInfo {
        name: "Domino Slider",
        summary: "Build a domino chain by matching the open number.",
        kind: GameKind::Micro(9),
    },
    GameInfo {
        name: "Tile Slider",
        summary: "Solve a shuffled 3x3 tile board in as few moves as possible.",
        kind: GameKind::Micro(10),
    },
    GameInfo {
        name: "Marble Labyrinth",
        summary: "Roll until blocked through a maze and plan each stop.",
        kind: GameKind::Micro(11),
    },
    GameInfo {
        name: "Tower Stack",
        summary: "Time moving slabs so each layer overlaps the last.",
        kind: GameKind::Micro(12),
    },
    GameInfo {
        name: "Laser Maze",
        summary: "Toggle laser nodes in plus patterns until every light is off.",
        kind: GameKind::Micro(13),
    },
    GameInfo {
        name: "Quantum Cups",
        summary: "Cross a scanner field where hidden hazards give nearby pings.",
        kind: GameKind::Micro(14),
    },
    GameInfo {
        name: "Blackjack Table",
        summary: "Hit, stand, and try to beat the dealer without busting.",
        kind: GameKind::Micro(15),
    },
    GameInfo {
        name: "Poker Rain",
        summary: "Draw a five-card terminal hand and score pairs, triples, or quads.",
        kind: GameKind::Micro(16),
    },
    GameInfo {
        name: "Blackjack Blitz",
        summary: "Draw up to five cards and bank as close to 21 as possible.",
        kind: GameKind::Micro(17),
    },
    GameInfo {
        name: "Go Territory",
        summary: "Claim territory stones across an open board before exiting.",
        kind: GameKind::Micro(18),
    },
    GameInfo {
        name: "Reversi Flip",
        summary: "Place discs to flip enemy lines on a 6x6 board.",
        kind: GameKind::Micro(19),
    },
    GameInfo {
        name: "Battleship Radar",
        summary: "Fire torpedoes at an 8x8 grid, using radar pings after misses.",
        kind: GameKind::Micro(20),
    },
    GameInfo {
        name: "Sea Chess",
        summary: "Shift sea lanes, collect flags, and dodge naval hazards.",
        kind: GameKind::Micro(21),
    },
    GameInfo {
        name: "Pinball Nudge",
        summary: "Time the flipper with Space to knock bumpers and avoid drains.",
        kind: GameKind::Micro(22),
    },
    GameInfo {
        name: "Skee Ball",
        summary: "Set lane and power, then roll for target rings.",
        kind: GameKind::Micro(23),
    },
    GameInfo {
        name: "Air Hockey",
        summary: "Slide between lanes to grab power shots and dodge loose pucks.",
        kind: GameKind::Micro(24),
    },
    GameInfo {
        name: "Mini Golf",
        summary: "Aim putts through wall gaps and sink the ball before strokes run out.",
        kind: GameKind::Micro(25),
    },
    GameInfo {
        name: "Bowling Lane",
        summary: "Aim and roll down a pin lane across ten frames.",
        kind: GameKind::Micro(26),
    },
    GameInfo {
        name: "Curling Slide",
        summary: "Set aim and weight, then slide stones toward the house.",
        kind: GameKind::Micro(27),
    },
    GameInfo {
        name: "Soccer Keeper",
        summary: "Slide across the goal line and block incoming shots.",
        kind: GameKind::Micro(28),
    },
    GameInfo {
        name: "Basket Toss",
        summary: "Set aim and power, then shoot through wind for points.",
        kind: GameKind::Micro(29),
    },
    GameInfo {
        name: "Hockey Break",
        summary: "Break through hockey lanes, collect pucks, and dodge checks.",
        kind: GameKind::Micro(30),
    },
    GameInfo {
        name: "Tennis Rally",
        summary: "Move into position and press Space to time each rally hit.",
        kind: GameKind::Micro(31),
    },
    GameInfo {
        name: "Cricket Catch",
        summary: "Line up and time catches while bouncers punish bad reads.",
        kind: GameKind::Micro(32),
    },
    GameInfo {
        name: "Darts Board",
        summary: "Aim at a dart board while wind nudges each throw.",
        kind: GameKind::Micro(33),
    },
    GameInfo {
        name: "Archery Range",
        summary: "Tune aim and draw power while wind pushes each shot.",
        kind: GameKind::Micro(34),
    },
    GameInfo {
        name: "Ski Slalom",
        summary: "Match the target gate lane before it reaches the finish.",
        kind: GameKind::Micro(35),
    },
    GameInfo {
        name: "Snowboard Rail",
        summary: "Collect sparks while W/S keeps the rail balance meter alive.",
        kind: GameKind::Micro(36),
    },
    GameInfo {
        name: "Skate Park",
        summary: "Input the shown trick combo before mistakes wipe the run.",
        kind: GameKind::Micro(37),
    },
    GameInfo {
        name: "BMX Alley",
        summary: "Change lanes, jump debris, and grab repair kits.",
        kind: GameKind::Micro(38),
    },
    GameInfo {
        name: "Horse Dash",
        summary: "Jump hazards and collect horseshoes before stamina runs dry.",
        kind: GameKind::Micro(39),
    },
    GameInfo {
        name: "Pirate Plunder",
        summary: "Collect treasure across a dangerous island map.",
        kind: GameKind::Micro(40),
    },
    GameInfo {
        name: "Ninja Rooftop",
        summary: "Dash lanes, jump rooftop hazards, and grab scrolls.",
        kind: GameKind::Micro(41),
    },
    GameInfo {
        name: "Samurai Path",
        summary: "Trace honor nodes through a guarded maze.",
        kind: GameKind::Micro(42),
    },
    GameInfo {
        name: "Robot Factory",
        summary: "Repeat the assembly sequence to build the bot cleanly.",
        kind: GameKind::Micro(43),
    },
    GameInfo {
        name: "Alien Orchard",
        summary: "Harvest glowing fruit while avoiding alien thorns.",
        kind: GameKind::Micro(44),
    },
    GameInfo {
        name: "Moon Miner",
        summary: "Shift mining lanes, collect ore, and dodge moon rocks.",
        kind: GameKind::Micro(45),
    },
    GameInfo {
        name: "Mars Rover",
        summary: "Use scanner pings to cross hidden hazards and reach the uplink.",
        kind: GameKind::Micro(46),
    },
    GameInfo {
        name: "Saturn Rings",
        summary: "Sail ring lanes, grab ice, and dodge fast debris.",
        kind: GameKind::Micro(47),
    },
    GameInfo {
        name: "Astro Farmer",
        summary: "Harvest enough orbit crops while avoiding pests.",
        kind: GameKind::Micro(48),
    },
    GameInfo {
        name: "Deep Sea Maze",
        summary: "Watch pressure and sonar pings while crossing hidden hazards.",
        kind: GameKind::Micro(49),
    },
    GameInfo {
        name: "Submarine Sweep",
        summary: "Shift submarine lanes, collect oxygen, and avoid mines.",
        kind: GameKind::Micro(50),
    },
    GameInfo {
        name: "Volcano Vault",
        summary: "Collect crystals and escape before the lava wins.",
        kind: GameKind::Micro(51),
    },
    GameInfo {
        name: "Jungle Relic",
        summary: "Find relics in a maze and slip past traps.",
        kind: GameKind::Micro(52),
    },
    GameInfo {
        name: "Desert Caravan",
        summary: "Cross dunes, grab water, and avoid heat mirages.",
        kind: GameKind::Micro(53),
    },
    GameInfo {
        name: "Castle Siege",
        summary: "Catch enough siege supplies while stones drain your lives.",
        kind: GameKind::Micro(54),
    },
    GameInfo {
        name: "Dragon Hoard",
        summary: "Collect treasure through a lair maze and reach the exit.",
        kind: GameKind::Micro(55),
    },
    GameInfo {
        name: "Wizard Duel",
        summary: "Cast the displayed spell chain before mistakes break the duel.",
        kind: GameKind::Micro(56),
    },
    GameInfo {
        name: "Potion Panic",
        summary: "Catch recipe potions in rhythm while smoke resets progress.",
        kind: GameKind::Micro(57),
    },
    GameInfo {
        name: "Time Tunnel",
        summary: "Race fast lanes, collect time sparks, and avoid timeline breaks.",
        kind: GameKind::Micro(58),
    },
    GameInfo {
        name: "Mirror Maze",
        summary: "Solve a sliding maze with left and right controls mirrored.",
        kind: GameKind::Micro(59),
    },
];

const MICRO_GAMES: &[MicroGame] = &[
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::ConnectFour,
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Quest(QuestKind::Checkmate),
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::WordGuess(WordKind::Vault),
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::Catch(CatchKind::Glyph),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Lane(LaneKind::Rune),
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::Quest(QuestKind::Cipher),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::WordGuess(WordKind::Hangman),
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Mancala,
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::MiniSudoku,
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::DominoChain,
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::SlidePuzzle,
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::Marble),
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::TowerStack,
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::LightsOut,
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Quest(QuestKind::Quantum),
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Blackjack,
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Catch(CatchKind::Poker),
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::BlackjackBlitz,
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Quest(QuestKind::Go),
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Reversi,
    },
    MicroGame {
        category: GameCategory::Strategy,
        mode: MicroMode::Battleship,
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Lane(LaneKind::Sea),
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::Catch(CatchKind::Pinball),
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::SkeeBall,
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::Lane(LaneKind::AirHockey),
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::MiniGolf,
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Bowling,
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::Aim(AimKind::Curling),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Keeper,
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Aim(AimKind::Basket),
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Lane(LaneKind::Hockey),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Catch(CatchKind::Tennis),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Catch(CatchKind::Cricket),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Darts,
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Aim(AimKind::Archery),
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Lane(LaneKind::Ski),
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Lane(LaneKind::Snowboard),
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Sequence(SequenceKind::Trick),
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Lane(LaneKind::Bmx),
    },
    MicroGame {
        category: GameCategory::Racing,
        mode: MicroMode::Lane(LaneKind::Horse),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::Pirate),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Lane(LaneKind::Ninja),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Quest(QuestKind::Samurai),
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::Sequence(SequenceKind::Factory),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Catch(CatchKind::Alien),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Lane(LaneKind::Moon),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::Mars),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Lane(LaneKind::Saturn),
    },
    MicroGame {
        category: GameCategory::Arcade,
        mode: MicroMode::Catch(CatchKind::Astro),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::DeepSea),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Lane(LaneKind::Submarine),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::Volcano),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::Jungle),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Lane(LaneKind::Desert),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Catch(CatchKind::Castle),
    },
    MicroGame {
        category: GameCategory::Adventure,
        mode: MicroMode::Quest(QuestKind::Dragon),
    },
    MicroGame {
        category: GameCategory::Action,
        mode: MicroMode::Sequence(SequenceKind::Duel),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Catch(CatchKind::Potion),
    },
    MicroGame {
        category: GameCategory::Reflex,
        mode: MicroMode::Lane(LaneKind::Time),
    },
    MicroGame {
        category: GameCategory::Puzzle,
        mode: MicroMode::Quest(QuestKind::Mirror),
    },
];

struct AppState {
    difficulty_index: usize,
    endless_mode: bool,
    pong_assist_index: usize,
    pong_speed_index: usize,
    theme_index: usize,
    glyph_index: usize,
    themes: Vec<Theme>,
    glyph_sets: Vec<GlyphSet>,
    app_title: String,
    scores: HashMap<String, u32>,
    favorites: HashSet<String>,
    sound_enabled: bool,
    click_effects: bool,
    controls: Controls,
    rng: Rng,
    last_sound: HashMap<&'static str, Instant>,
    last_any_sound: Option<Instant>,
    sound_child: Option<Child>,
}

impl AppState {
    fn difficulty(&self) -> &Difficulty {
        &DIFFICULTIES[self.difficulty_index]
    }

    fn starting_lives(&self) -> i32 {
        if self.endless_mode {
            999
        } else {
            self.difficulty().lives
        }
    }

    fn pong_assist_name(&self) -> &'static str {
        PONG_ASSIST_NAMES[self.pong_assist_index % PONG_ASSIST_NAMES.len()]
    }

    fn pong_speed_name(&self) -> &'static str {
        PONG_SPEED_NAMES[self.pong_speed_index % PONG_SPEED_NAMES.len()]
    }

    fn pong_speed_factor(&self) -> f64 {
        PONG_SPEED_FACTORS[self.pong_speed_index % PONG_SPEED_FACTORS.len()]
    }

    fn theme(&self) -> &Theme {
        &self.themes[self.theme_index % self.themes.len()]
    }

    fn glyphs(&self) -> &GlyphSet {
        &self.glyph_sets[self.glyph_index % self.glyph_sets.len()]
    }

    fn custom_theme_mut(&mut self) -> &mut Theme {
        let custom_index = self.themes.len() - 1;
        self.theme_index = custom_index;
        save_theme_index(custom_index);
        &mut self.themes[custom_index]
    }
}

struct Terminal {
    original: String,
}

impl Terminal {
    fn enter() -> io::Result<Self> {
        let original = Command::new("stty")
            .arg("-g")
            .output()
            .map(|out| String::from_utf8_lossy(&out.stdout).trim().to_string())?;
        let _ = Command::new("stty")
            .args(["raw", "-echo", "min", "0", "time", "0"])
            .status();
        print!("\x1b[?1049h\x1b[?25l\x1b[2J\x1b[H");
        io::stdout().flush()?;
        Ok(Self { original })
    }
}

impl Drop for Terminal {
    fn drop(&mut self) {
        print!("\x1b[0m\x1b[?25h\x1b[?1049l");
        let _ = io::stdout().flush();
        if !self.original.is_empty() {
            let _ = Command::new("stty").arg(&self.original).status();
        }
    }
}

struct Rng {
    state: u64,
}

impl Rng {
    fn new() -> Self {
        let nanos = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos() as u64;
        let pid = std::process::id() as u64;
        Self {
            state: nanos ^ pid.rotate_left(17) ^ 0x9E3779B97F4A7C15,
        }
    }

    fn next_u32(&mut self) -> u32 {
        self.state ^= self.state << 7;
        self.state ^= self.state >> 9;
        self.state ^= self.state << 8;
        (self.state >> 16) as u32
    }

    fn range(&mut self, min: i32, max: i32) -> i32 {
        if max <= min {
            return min;
        }
        min + (self.next_u32() % ((max - min + 1) as u32)) as i32
    }

    fn usize(&mut self, max: usize) -> usize {
        if max == 0 {
            0
        } else {
            (self.next_u32() as usize) % max
        }
    }

    fn chance(&mut self, numerator: u32, denominator: u32) -> bool {
        denominator > 0 && self.next_u32() % denominator < numerator
    }
}

fn main() {
    if let Err(error) = run() {
        eprintln!("games error: {error}");
        std::process::exit(1);
    }
}

fn run() -> io::Result<()> {
    let _terminal = Terminal::enter()?;
    let themes = load_themes();
    let theme_index = load_theme_index(themes.len());
    let glyph_sets = load_glyph_sets();
    let glyph_index = load_glyph_index(glyph_sets.len());
    let controls = load_controls();
    sync_controls(controls);
    let (pong_assist_index, pong_speed_index) = load_pong_options();
    let mut state = AppState {
        difficulty_index: load_difficulty_index(),
        endless_mode: load_endless_mode(),
        pong_assist_index,
        pong_speed_index,
        theme_index,
        glyph_index,
        themes,
        glyph_sets,
        app_title: load_app_title(),
        scores: load_scores(),
        favorites: load_favorites(),
        sound_enabled: load_sound_enabled(),
        click_effects: load_click_effects(),
        controls,
        rng: Rng::new(),
        last_sound: HashMap::new(),
        last_any_sound: None,
        sound_child: None,
    };
    home_menu(&mut state)
}

fn load_glyph_sets() -> Vec<GlyphSet> {
    vec![
        GlyphSet {
            name: "Classic",
            description: "ASCII arcade lines.",
            top_left: "+",
            top_right: "+",
            bottom_left: "+",
            bottom_right: "+",
            horizontal: "-",
            vertical: "|",
            title_left: "[",
            title_right: "]",
            selector: ">",
            scroll_up: "^",
            scroll_down: "v",
            scroll_track: "|",
            scroll_thumb: "#",
            button_left: "[",
            button_right: "]",
        },
        GlyphSet {
            name: "Rounded",
            description: "Soft terminal panels.",
            top_left: "╭",
            top_right: "╮",
            bottom_left: "╰",
            bottom_right: "╯",
            horizontal: "─",
            vertical: "│",
            title_left: "┤",
            title_right: "├",
            selector: "›",
            scroll_up: "▲",
            scroll_down: "▼",
            scroll_track: "│",
            scroll_thumb: "●",
            button_left: "‹",
            button_right: "›",
        },
        GlyphSet {
            name: "Double",
            description: "Heavy cabinet chrome.",
            top_left: "╔",
            top_right: "╗",
            bottom_left: "╚",
            bottom_right: "╝",
            horizontal: "═",
            vertical: "║",
            title_left: "╡",
            title_right: "╞",
            selector: "▶",
            scroll_up: "⇧",
            scroll_down: "⇩",
            scroll_track: "║",
            scroll_thumb: "█",
            button_left: "❮",
            button_right: "❯",
        },
        GlyphSet {
            name: "Circuit",
            description: "Sharp neon tracework.",
            top_left: "┏",
            top_right: "┓",
            bottom_left: "┗",
            bottom_right: "┛",
            horizontal: "━",
            vertical: "┃",
            title_left: "╾",
            title_right: "╼",
            selector: "▹",
            scroll_up: "△",
            scroll_down: "▽",
            scroll_track: "┆",
            scroll_thumb: "◆",
            button_left: "◁",
            button_right: "▷",
        },
        GlyphSet {
            name: "Arcane",
            description: "Runic fantasy ornament.",
            top_left: "◜",
            top_right: "◝",
            bottom_left: "◟",
            bottom_right: "◞",
            horizontal: "═",
            vertical: "║",
            title_left: "◇",
            title_right: "◇",
            selector: "◆",
            scroll_up: "⬖",
            scroll_down: "⬘",
            scroll_track: "╎",
            scroll_thumb: "◈",
            button_left: "⟦",
            button_right: "⟧",
        },
        GlyphSet {
            name: "Starlace",
            description: "Bright ornamental sparks.",
            top_left: "✦",
            top_right: "✦",
            bottom_left: "✧",
            bottom_right: "✧",
            horizontal: "·",
            vertical: "┊",
            title_left: "✶",
            title_right: "✶",
            selector: "✦",
            scroll_up: "✧",
            scroll_down: "✧",
            scroll_track: "┊",
            scroll_thumb: "✹",
            button_left: "✧",
            button_right: "✧",
        },
        GlyphSet {
            name: "Mythic",
            description: "Exotic angular relics.",
            top_left: "◢",
            top_right: "◣",
            bottom_left: "◥",
            bottom_right: "◤",
            horizontal: "▔",
            vertical: "▏",
            title_left: "◀",
            title_right: "▶",
            selector: "▰",
            scroll_up: "▴",
            scroll_down: "▾",
            scroll_track: "▏",
            scroll_thumb: "▣",
            button_left: "◀",
            button_right: "▶",
        },
    ]
}

fn load_themes() -> Vec<Theme> {
    let mut themes = vec![
        Theme {
            name: "Neon".to_string(),
            fg: 15,
            bg: Some(0),
            title: 14,
            accent: 10,
            secondary: 13,
            danger: 9,
            success: 10,
            muted: 8,
            highlight: 11,
        },
        Theme {
            name: "Amber".to_string(),
            fg: 15,
            bg: Some(0),
            title: 11,
            accent: 3,
            secondary: 10,
            danger: 9,
            success: 2,
            muted: 8,
            highlight: 15,
        },
        Theme {
            name: "Ocean".to_string(),
            fg: 15,
            bg: Some(0),
            title: 14,
            accent: 12,
            secondary: 6,
            danger: 9,
            success: 10,
            muted: 8,
            highlight: 15,
        },
        Theme {
            name: "Candy".to_string(),
            fg: 15,
            bg: Some(0),
            title: 13,
            accent: 11,
            secondary: 14,
            danger: 9,
            success: 10,
            muted: 8,
            highlight: 15,
        },
        Theme {
            name: "Forest".to_string(),
            fg: 15,
            bg: Some(0),
            title: 10,
            accent: 2,
            secondary: 11,
            danger: 9,
            success: 14,
            muted: 8,
            highlight: 15,
        },
        Theme {
            name: "Mono".to_string(),
            fg: 15,
            bg: Some(0),
            title: 15,
            accent: 7,
            secondary: 8,
            danger: 9,
            success: 10,
            muted: 8,
            highlight: 0,
        },
    ];
    themes.extend(load_saved_themes());
    themes.push(load_custom_theme().unwrap_or_else(|| {
        let mut custom = themes[0].clone();
        custom.name = "Custom".to_string();
        custom
    }));
    themes
}

fn home_dir() -> PathBuf {
    env::var_os("HOME")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from("."))
}

fn scores_path() -> PathBuf {
    home_dir().join(".tui_arcade_scores.json")
}

fn theme_path() -> PathBuf {
    home_dir().join(".tui_arcade_theme_rust.txt")
}

fn saved_themes_path() -> PathBuf {
    home_dir().join(".tui_arcade_saved_themes.txt")
}

fn theme_index_path() -> PathBuf {
    home_dir().join(".tui_arcade_theme_index.txt")
}

fn difficulty_path() -> PathBuf {
    home_dir().join(".tui_arcade_difficulty.txt")
}

fn endless_path() -> PathBuf {
    home_dir().join(".tui_arcade_endless.txt")
}

fn sound_path() -> PathBuf {
    home_dir().join(".tui_arcade_sound.txt")
}

fn click_effects_path() -> PathBuf {
    home_dir().join(".tui_arcade_click_effects.txt")
}

fn pong_options_path() -> PathBuf {
    home_dir().join(".tui_arcade_pong_options.txt")
}

fn glyph_path() -> PathBuf {
    home_dir().join(".tui_arcade_glyphs.txt")
}

fn controls_path() -> PathBuf {
    home_dir().join(".tui_arcade_controls.txt")
}

fn title_path() -> PathBuf {
    home_dir().join(".tui_arcade_title.txt")
}

fn favorites_path() -> PathBuf {
    home_dir().join(".tui_arcade_favorites.txt")
}

fn load_glyph_index(len: usize) -> usize {
    fs::read_to_string(glyph_path())
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .map(|index| index % len.max(1))
        .unwrap_or(0)
}

fn load_theme_index(len: usize) -> usize {
    fs::read_to_string(theme_index_path())
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .map(|index| index % len.max(1))
        .unwrap_or(0)
}

fn save_theme_index(index: usize) {
    let _ = fs::write(theme_index_path(), format!("{index}\n"));
}

fn save_glyph_index(index: usize) {
    let _ = fs::write(glyph_path(), format!("{index}\n"));
}

fn load_difficulty_index() -> usize {
    fs::read_to_string(difficulty_path())
        .ok()
        .and_then(|text| text.trim().parse::<usize>().ok())
        .map(|index| index % DIFFICULTIES.len())
        .unwrap_or(1)
}

fn save_difficulty_index(index: usize) {
    let _ = fs::write(difficulty_path(), format!("{index}\n"));
}

fn load_bool(path: PathBuf, default: bool) -> bool {
    fs::read_to_string(path)
        .ok()
        .map(|text| matches!(text.trim(), "1" | "true" | "on" | "yes"))
        .unwrap_or(default)
}

fn save_bool(path: PathBuf, enabled: bool) {
    let _ = fs::write(path, if enabled { "1\n" } else { "0\n" });
}

fn load_endless_mode() -> bool {
    load_bool(endless_path(), false)
}

fn save_endless_mode(enabled: bool) {
    save_bool(endless_path(), enabled);
}

fn load_sound_enabled() -> bool {
    load_bool(sound_path(), true)
}

fn save_sound_enabled(enabled: bool) {
    save_bool(sound_path(), enabled);
}

fn load_click_effects() -> bool {
    load_bool(click_effects_path(), true)
}

fn save_click_effects(enabled: bool) {
    save_bool(click_effects_path(), enabled);
}

fn load_pong_options() -> (usize, usize) {
    let mut assist = 1usize;
    let mut speed = 0usize;
    if let Ok(text) = fs::read_to_string(pong_options_path()) {
        for line in text.lines() {
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key {
                "assist" => {
                    assist =
                        value.trim().parse::<usize>().unwrap_or(assist) % PONG_ASSIST_NAMES.len();
                }
                "speed" => {
                    speed = value.trim().parse::<usize>().unwrap_or(speed) % PONG_SPEED_NAMES.len();
                }
                _ => {}
            }
        }
    }
    (assist, speed)
}

fn save_pong_options(assist: usize, speed: usize) {
    let text = format!(
        "assist={}\nspeed={}\n",
        assist % PONG_ASSIST_NAMES.len(),
        speed % PONG_SPEED_NAMES.len()
    );
    let _ = fs::write(pong_options_path(), text);
}

fn load_controls() -> Controls {
    let mut controls = Controls::default();
    let Ok(text) = fs::read_to_string(controls_path()) else {
        return controls;
    };
    for line in text.lines() {
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        let Some(ch) = parse_control_value(value) else {
            continue;
        };
        match key {
            "up" => controls.up = ch,
            "down" => controls.down = ch,
            "left" => controls.left = ch,
            "right" => controls.right = ch,
            "action" => controls.action = ch,
            "pause" => controls.pause = ch,
            "quit" => controls.quit = ch,
            _ => {}
        }
    }
    if controls_unique(controls) {
        controls
    } else {
        Controls::default()
    }
}

fn save_controls(controls: Controls) {
    let text = format!(
        "up={}\ndown={}\nleft={}\nright={}\naction={}\npause={}\nquit={}\n",
        serialize_control_value(controls.up),
        serialize_control_value(controls.down),
        serialize_control_value(controls.left),
        serialize_control_value(controls.right),
        serialize_control_value(controls.action),
        serialize_control_value(controls.pause),
        serialize_control_value(controls.quit)
    );
    let _ = fs::write(controls_path(), text);
}

fn parse_control_value(value: &str) -> Option<char> {
    let value = value.trim();
    if value.eq_ignore_ascii_case("space") {
        Some(' ')
    } else {
        value.chars().next().map(|ch| ch.to_ascii_lowercase())
    }
}

fn serialize_control_value(ch: char) -> String {
    if ch == ' ' {
        "space".to_string()
    } else {
        ch.to_string()
    }
}

fn control_label(ch: char) -> String {
    match ch {
        ' ' => "Space".to_string(),
        '\0' => "Unbound".to_string(),
        other => other.to_ascii_uppercase().to_string(),
    }
}

fn controls_unique(controls: Controls) -> bool {
    let values = [
        controls.up,
        controls.down,
        controls.left,
        controls.right,
        controls.action,
        controls.pause,
        controls.quit,
    ];
    values.iter().enumerate().all(|(index, value)| {
        *value != '\0' && values.iter().skip(index + 1).all(|other| other != value)
    })
}

fn sync_controls(controls: Controls) {
    let lock = ACTIVE_CONTROLS.get_or_init(|| Mutex::new(Controls::default()));
    if let Ok(mut active) = lock.lock() {
        *active = controls;
    }
}

fn active_controls() -> Controls {
    ACTIVE_CONTROLS
        .get_or_init(|| Mutex::new(Controls::default()))
        .lock()
        .map(|controls| *controls)
        .unwrap_or_default()
}

fn load_app_title() -> String {
    fs::read_to_string(title_path())
        .ok()
        .map(|text| text.trim().to_string())
        .filter(|text| !text.is_empty())
        .unwrap_or_else(|| DEFAULT_TITLE.to_string())
}

fn save_app_title(title: &str) {
    let _ = fs::write(title_path(), format!("{}\n", title.trim()));
}

fn load_scores() -> HashMap<String, u32> {
    let mut scores = HashMap::new();
    let Ok(text) = fs::read_to_string(scores_path()) else {
        return scores;
    };
    let bytes = text.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] != b'"' {
            i += 1;
            continue;
        }
        let start = i + 1;
        i = start;
        while i < bytes.len() && bytes[i] != b'"' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        let key = String::from_utf8_lossy(&bytes[start..i])
            .replace("\\\"", "\"")
            .replace("\\\\", "\\");
        i += 1;
        while i < bytes.len() && bytes[i] != b':' {
            i += 1;
        }
        if i >= bytes.len() {
            break;
        }
        i += 1;
        while i < bytes.len() && !bytes[i].is_ascii_digit() {
            i += 1;
        }
        let n_start = i;
        while i < bytes.len() && bytes[i].is_ascii_digit() {
            i += 1;
        }
        if let Ok(value) = String::from_utf8_lossy(&bytes[n_start..i]).parse::<u32>() {
            scores.insert(key, value);
        }
    }
    scores
}

fn save_scores(scores: &HashMap<String, u32>) {
    let mut pairs: Vec<_> = scores.iter().collect();
    pairs.sort_by(|a, b| a.0.cmp(b.0));
    let mut out = String::from("{\n");
    for (index, (name, score)) in pairs.iter().enumerate() {
        let comma = if index + 1 == pairs.len() { "" } else { "," };
        out.push_str(&format!(
            "  \"{}\": {}{}\n",
            escape_json(name),
            score,
            comma
        ));
    }
    out.push_str("}\n");
    let _ = fs::write(scores_path(), out);
}

fn escape_json(input: &str) -> String {
    input.replace('\\', "\\\\").replace('"', "\\\"")
}

fn record_score(state: &mut AppState, game: &str, score: u32) {
    let entry = state.scores.entry(game.to_string()).or_insert(0);
    if score > *entry {
        *entry = score;
        save_scores(&state.scores);
    }
}

fn erase_scores(state: &mut AppState) -> bool {
    state.scores.clear();
    match fs::remove_file(scores_path()) {
        Ok(_) => true,
        Err(error) if error.kind() == io::ErrorKind::NotFound => true,
        Err(_) => false,
    }
}

fn load_favorites() -> HashSet<String> {
    let valid_names: HashSet<&str> = GAMES.iter().map(|game| game.name).collect();
    fs::read_to_string(favorites_path())
        .ok()
        .map(|text| {
            text.lines()
                .map(str::trim)
                .filter(|name| valid_names.contains(name))
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn save_favorites(favorites: &HashSet<String>) {
    let mut names: Vec<&str> = GAMES
        .iter()
        .map(|game| game.name)
        .filter(|name| favorites.contains(*name))
        .collect();
    names.sort_unstable();
    let text = if names.is_empty() {
        String::new()
    } else {
        format!("{}\n", names.join("\n"))
    };
    let _ = fs::write(favorites_path(), text);
}

fn toggle_favorite(state: &mut AppState, name: &str) -> bool {
    let enabled = if state.favorites.contains(name) {
        state.favorites.remove(name);
        false
    } else {
        state.favorites.insert(name.to_string());
        true
    };
    save_favorites(&state.favorites);
    enabled
}

fn load_custom_theme() -> Option<Theme> {
    let text = fs::read_to_string(theme_path()).ok()?;
    let mut theme = default_theme("Custom");
    for line in text.lines() {
        apply_theme_line(&mut theme, line);
    }
    Some(theme)
}

fn default_theme(name: &str) -> Theme {
    Theme {
        name: name.to_string(),
        fg: 15,
        bg: Some(0),
        title: 14,
        accent: 10,
        secondary: 13,
        danger: 9,
        success: 10,
        muted: 8,
        highlight: 11,
    }
}

fn apply_theme_line(theme: &mut Theme, line: &str) {
    let Some((key, value)) = line.split_once('=') else {
        return;
    };
    match key.trim() {
        "fg" => theme.fg = parse_theme_color(value, theme.fg),
        "bg" => theme.bg = parse_bg(value.trim()),
        "title" => theme.title = parse_theme_color(value, theme.title),
        "accent" => theme.accent = parse_theme_color(value, theme.accent),
        "secondary" => theme.secondary = parse_theme_color(value, theme.secondary),
        "danger" => theme.danger = parse_theme_color(value, theme.danger),
        "success" => theme.success = parse_theme_color(value, theme.success),
        "muted" => theme.muted = parse_theme_color(value, theme.muted),
        "highlight" => theme.highlight = parse_theme_color(value, theme.highlight),
        _ => {}
    }
}

fn parse_theme_color(value: &str, fallback: u8) -> u8 {
    value
        .trim()
        .parse::<u8>()
        .ok()
        .filter(|color| *color < COLOR_NAMES.len() as u8)
        .unwrap_or(fallback)
}

fn parse_bg(value: &str) -> Option<u8> {
    if value == "none" {
        None
    } else {
        value
            .parse()
            .ok()
            .filter(|color| *color < COLOR_NAMES.len() as u8)
    }
}

fn theme_storage_text(theme: &Theme) -> String {
    let bg = theme
        .bg
        .map(|v| v.to_string())
        .unwrap_or_else(|| "none".to_string());
    format!(
        "fg={}\nbg={}\ntitle={}\naccent={}\nsecondary={}\ndanger={}\nsuccess={}\nmuted={}\nhighlight={}\n",
        theme.fg, bg, theme.title, theme.accent, theme.secondary, theme.danger, theme.success, theme.muted, theme.highlight
    )
}

fn save_custom_theme(theme: &Theme) {
    let _ = fs::write(theme_path(), theme_storage_text(theme));
}

fn load_saved_theme_slots() -> Vec<Option<Theme>> {
    let mut slots = vec![None; SAVED_THEME_SLOTS];
    let Ok(text) = fs::read_to_string(saved_themes_path()) else {
        return slots;
    };
    let mut current_slot: Option<usize> = None;
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.starts_with("[Saved ") && trimmed.ends_with(']') {
            let slot = trimmed
                .trim_start_matches("[Saved ")
                .trim_end_matches(']')
                .parse::<usize>()
                .ok()
                .and_then(|number| number.checked_sub(1))
                .filter(|slot| *slot < SAVED_THEME_SLOTS);
            current_slot = slot;
            if let Some(slot) = slot {
                slots[slot] = Some(default_theme(&format!("Saved {}", slot + 1)));
            }
            continue;
        }
        if let Some(slot) = current_slot {
            if let Some(theme) = &mut slots[slot] {
                apply_theme_line(theme, trimmed);
            }
        }
    }
    slots
}

fn load_saved_themes() -> Vec<Theme> {
    load_saved_theme_slots().into_iter().flatten().collect()
}

fn save_saved_theme_slots(slots: &[Option<Theme>]) -> bool {
    let mut text = String::new();
    for (index, theme) in slots.iter().enumerate() {
        if let Some(theme) = theme {
            text.push_str(&format!("[Saved {}]\n", index + 1));
            text.push_str(&theme_storage_text(theme));
            text.push('\n');
        }
    }
    fs::write(saved_themes_path(), text).is_ok()
}

fn save_theme_slot(state: &mut AppState, slot: usize) -> bool {
    if slot >= SAVED_THEME_SLOTS {
        return false;
    }
    let mut slots = load_saved_theme_slots();
    let mut saved = state.theme().clone();
    saved.name = format!("Saved {}", slot + 1);
    slots[slot] = Some(saved);
    if !save_saved_theme_slots(&slots) {
        return false;
    }
    let saved_name = format!("Saved {}", slot + 1);
    state.themes = load_themes();
    if let Some(index) = state
        .themes
        .iter()
        .position(|theme| theme.name == saved_name)
    {
        state.theme_index = index;
        save_theme_index(index);
    }
    true
}

fn terminal_size() -> (usize, usize) {
    let output = Command::new("stty")
        .arg("size")
        .stdin(Stdio::inherit())
        .output();
    if let Ok(output) = output {
        let text = String::from_utf8_lossy(&output.stdout);
        let mut parts = text.split_whitespace();
        if let (Some(rows), Some(cols)) = (parts.next(), parts.next()) {
            if let (Ok(rows), Ok(cols)) = (rows.parse::<usize>(), cols.parse::<usize>()) {
                return (rows.max(1), cols.max(1));
            }
        }
    }
    (24, 80)
}

fn size_changed(last_size: &mut (usize, usize)) -> bool {
    let size = terminal_size();
    if size != *last_size {
        *last_size = size;
        true
    } else {
        false
    }
}

fn full_board(min_w: i32, min_h: i32, max_w: i32, max_h: i32) -> (i32, i32) {
    let (rows, cols) = terminal_size();
    let width = cols
        .saturating_sub(8)
        .max(min_w as usize)
        .min(max_w as usize);
    let height = rows
        .saturating_sub(6)
        .max(min_h as usize)
        .min(max_h as usize);
    (width as i32, height as i32)
}

fn fg_code(color: u8) -> u8 {
    if color < 8 {
        30 + color
    } else {
        90 + (color - 8)
    }
}

fn bg_code(color: u8) -> u8 {
    if color < 8 {
        40 + color
    } else {
        100 + (color - 8)
    }
}

fn role_color(theme: &Theme, role: Role) -> u8 {
    match role {
        Role::Normal => theme.fg,
        Role::Title => theme.title,
        Role::Accent => theme.accent,
        Role::Secondary => theme.secondary,
        Role::Danger => theme.danger,
        Role::Success => theme.success,
        Role::Muted => theme.muted,
        Role::Highlight => theme.highlight,
    }
}

fn style(theme: &Theme, role: Role, bold: bool, inverse: bool) -> String {
    let fg = role_color(theme, role);
    let bg = if inverse {
        Some(theme.highlight)
    } else {
        theme.bg
    };
    let mut codes = vec![fg_code(fg).to_string()];
    if let Some(bg) = bg {
        codes.push(bg_code(bg).to_string());
    }
    if bold {
        codes.push("1".to_string());
    }
    if inverse {
        codes.push("7".to_string());
    }
    format!("\x1b[{}m", codes.join(";"))
}

fn reset() -> &'static str {
    "\x1b[0m"
}

fn goto(row: usize, col: usize) -> String {
    format!("\x1b[{};{}H", row + 1, col + 1)
}

fn clear_buf(buf: &mut String, theme: &Theme) {
    buf.push_str("\x1b[0m\x1b[2J\x1b[H");
    if let Some(bg) = theme.bg {
        buf.push_str(&format!("\x1b[{}m", bg_code(bg)));
    }
}

fn put(
    buf: &mut String,
    row: usize,
    col: usize,
    text: &str,
    theme: &Theme,
    role: Role,
    bold: bool,
) {
    buf.push_str(&goto(row, col));
    buf.push_str(&style(theme, role, bold, false));
    buf.push_str(text);
    buf.push_str(reset());
}

fn put_inv(buf: &mut String, row: usize, col: usize, text: &str, theme: &Theme, role: Role) {
    buf.push_str(&goto(row, col));
    buf.push_str(&style(theme, role, true, true));
    buf.push_str(text);
    buf.push_str(reset());
}

fn center(
    buf: &mut String,
    row: usize,
    text: &str,
    theme: &Theme,
    role: Role,
    bold: bool,
    width: usize,
) {
    let col = width.saturating_sub(visible_width(text)) / 2;
    put(buf, row, col, text, theme, role, bold);
}

fn draw_box(
    buf: &mut String,
    row: usize,
    col: usize,
    height: usize,
    width: usize,
    title: &str,
    theme: &Theme,
    role: Role,
    glyphs: &GlyphSet,
) {
    if height < 2 || width < 2 {
        return;
    }
    let top = format!(
        "{}{}{}",
        glyphs.top_left,
        repeat_glyph(glyphs.horizontal, width - 2),
        glyphs.top_right
    );
    let mid = format!(
        "{}{}{}",
        glyphs.vertical,
        " ".repeat(width - 2),
        glyphs.vertical
    );
    let bottom = format!(
        "{}{}{}",
        glyphs.bottom_left,
        repeat_glyph(glyphs.horizontal, width - 2),
        glyphs.bottom_right
    );
    put(buf, row, col, &top, theme, role, false);
    for y in 1..height - 1 {
        put(buf, row + y, col, &mid, theme, role, false);
    }
    put(buf, row + height - 1, col, &bottom, theme, role, false);
    let title_text = format!("{} {} {}", glyphs.title_left, title, glyphs.title_right);
    if !title.is_empty() && visible_width(&title_text) + 2 < width {
        put(buf, row, col + 2, &title_text, theme, role, true);
    }
}

fn draw_scrollbar(
    buf: &mut String,
    row: usize,
    col: usize,
    height: usize,
    total: usize,
    visible: usize,
    start: usize,
    theme: &Theme,
    role: Role,
    glyphs: &GlyphSet,
) {
    if height < 3 || visible >= total {
        return;
    }
    put(buf, row, col, glyphs.scroll_up, theme, role, true);
    put(
        buf,
        row + height - 1,
        col,
        glyphs.scroll_down,
        theme,
        role,
        true,
    );
    let track_h = height - 2;
    let thumb_h = ((track_h * visible) / total).max(1).min(track_h);
    let max_start = total.saturating_sub(visible).max(1);
    let thumb_top = start.saturating_mul(track_h.saturating_sub(thumb_h)) / max_start;
    for y in 0..track_h {
        let glyph = if y >= thumb_top && y < thumb_top + thumb_h {
            glyphs.scroll_thumb
        } else {
            glyphs.scroll_track
        };
        put(buf, row + 1 + y, col, glyph, theme, role, false);
    }
}

fn flush(buf: &str) {
    print!("{buf}");
    let _ = io::stdout().flush();
}

fn read_key() -> Option<Key> {
    read_key_inner(false)
}

fn read_text_key() -> Option<Key> {
    read_key_inner(true)
}

fn read_key_inner(preserve_case: bool) -> Option<Key> {
    let mut first = [0u8; 1];
    let Ok(count) = io::stdin().read(&mut first) else {
        return None;
    };
    if count == 0 {
        return None;
    }
    match first[0] {
        b'\r' | b'\n' => Some(Key::Enter),
        b' ' => {
            if preserve_case || active_controls().action == ' ' {
                Some(Key::Space)
            } else {
                Some(Key::Char('\0'))
            }
        }
        8 | 127 => Some(Key::Backspace),
        27 => {
            thread::sleep(Duration::from_millis(1));
            let mut rest = [0u8; 8];
            let count = io::stdin().read(&mut rest).unwrap_or(0);
            if count >= 2 && rest[0] == b'[' {
                match rest[1] {
                    b'A' => Some(Key::Up),
                    b'B' => Some(Key::Down),
                    b'C' => Some(Key::Right),
                    b'D' => Some(Key::Left),
                    _ => Some(Key::Esc),
                }
            } else {
                Some(Key::Esc)
            }
        }
        b => {
            let ch = b as char;
            if preserve_case {
                Some(Key::Char(ch))
            } else {
                Some(translate_control_char(ch.to_ascii_lowercase()))
            }
        }
    }
}

fn translate_control_char(ch: char) -> Key {
    let controls = active_controls();
    if ch == controls.up {
        Key::Char('w')
    } else if ch == controls.down {
        Key::Char('s')
    } else if ch == controls.left {
        Key::Char('a')
    } else if ch == controls.right {
        Key::Char('d')
    } else if ch == controls.action {
        Key::Space
    } else if ch == controls.pause {
        Key::Char('p')
    } else if ch == controls.quit {
        Key::Char('q')
    } else if matches!(ch, 'w' | 's' | 'a' | 'd' | 'p' | 'q') {
        Key::Char('\0')
    } else {
        Key::Char(ch)
    }
}

fn is_quit(key: Key) -> bool {
    matches!(key, Key::Esc | Key::Char('q'))
}

fn is_pause(key: Key) -> bool {
    matches!(key, Key::Char('p'))
}

fn sleep_frame(start: Instant, frame_ms: u64) {
    let frame = Duration::from_millis(frame_ms);
    let elapsed = start.elapsed();
    if elapsed < frame {
        thread::sleep(frame - elapsed);
    }
}

fn play_sound(state: &mut AppState, kind: &'static str) {
    if !state.sound_enabled {
        return;
    }
    let gap_ms = sound_gap_ms(kind);
    let global_gap = if matches!(kind, "click" | "paddle" | "wall") {
        85
    } else {
        125
    };
    let now = Instant::now();
    if state
        .last_sound
        .get(kind)
        .is_some_and(|last| now.duration_since(*last) < Duration::from_millis(gap_ms))
    {
        return;
    }
    if state
        .last_any_sound
        .is_some_and(|last| now.duration_since(last) < Duration::from_millis(global_gap))
    {
        return;
    }
    if let Some(child) = state.sound_child.as_mut() {
        match child.try_wait() {
            Ok(Some(_)) => {
                state.sound_child = None;
            }
            Ok(None) => return,
            Err(_) => {
                state.sound_child = None;
            }
        }
    }
    state.last_sound.insert(kind, now);
    state.last_any_sound = Some(now);
    let sound = match kind {
        "click" => "/System/Library/Sounds/Tink.aiff",
        "paddle" => "/System/Library/Sounds/Pop.aiff",
        "wall" => "/System/Library/Sounds/Funk.aiff",
        "score" => "/System/Library/Sounds/Glass.aiff",
        "alert" => "/System/Library/Sounds/Sosumi.aiff",
        _ => "/System/Library/Sounds/Pop.aiff",
    };
    if PathBuf::from(sound).exists() {
        state.sound_child = Command::new("afplay")
            .arg(sound)
            .stdout(Stdio::null())
            .stderr(Stdio::null())
            .spawn()
            .ok();
    } else {
        print!("\x07");
        let _ = io::stdout().flush();
    }
}

fn sound_gap_ms(kind: &str) -> u64 {
    match kind {
        "score" => 220,
        "wall" => 130,
        "paddle" => 95,
        "alert" => 280,
        "click" => 120,
        _ => 120,
    }
}

fn click_effect(state: &mut AppState, label: &str) {
    play_sound(state, "click");
    if !state.click_effects {
        return;
    }
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let glyphs = state.glyphs();
    let mut buf = String::new();
    let text = format!("{} {label} {}", glyphs.button_left, glyphs.button_right);
    put_inv(
        &mut buf,
        rows.saturating_sub(2),
        cols.saturating_sub(visible_width(&text)) / 2,
        &text,
        &theme,
        Role::Accent,
    );
    flush(&buf);
    thread::sleep(Duration::from_millis(55));
}

fn pause_screen(state: &mut AppState) -> Option<Duration> {
    let started = Instant::now();
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            let (rows, cols) = terminal_size();
            let theme = state.theme().clone();
            let glyphs = state.glyphs();
            let box_w = 52usize.min(cols.saturating_sub(4)).max(30);
            let box_h = 8usize;
            let top = rows.saturating_sub(box_h) / 2;
            let left = cols.saturating_sub(box_w) / 2;
            let mut buf = String::new();
            clear_buf(&mut buf, &theme);
            draw_box(
                &mut buf,
                top,
                left,
                box_h,
                box_w,
                "PAUSED",
                &theme,
                Role::Title,
                glyphs,
            );
            center(
                &mut buf,
                top + 2,
                "Game paused.",
                &theme,
                Role::Accent,
                true,
                cols,
            );
            center(
                &mut buf,
                top + 4,
                "Press P, Enter, or Space to resume.",
                &theme,
                Role::Normal,
                false,
                cols,
            );
            center(
                &mut buf,
                top + 5,
                "Press Q or Esc for menu.",
                &theme,
                Role::Muted,
                false,
                cols,
            );
            flush(&buf);
            dirty = false;
        }
        if let Some(key) = read_key() {
            if is_quit(key) {
                return None;
            }
            if is_pause(key) || key == Key::Enter || key == Key::Space {
                return Some(started.elapsed());
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn require_size(state: &AppState, min_rows: usize, min_cols: usize, title: &str) -> bool {
    let (rows, cols) = terminal_size();
    if rows >= min_rows && cols >= min_cols {
        return true;
    }
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        rows / 2 - 2,
        title,
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2,
        &format!("Resize terminal to at least {min_cols}x{min_rows}."),
        &theme,
        Role::Danger,
        false,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 2,
        "Press Q or Esc.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
    loop {
        if let Some(key) = read_key() {
            if is_quit(key) || key == Key::Enter {
                return false;
            }
        }
        thread::sleep(Duration::from_millis(30));
    }
}

fn wait_menu(state: &mut AppState, title: &str, lines: &[String], restart: bool) -> bool {
    let mut selected = 0usize;
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        let options = if restart {
            ["Restart", "Menu"]
        } else {
            ["Menu", "Menu"]
        };
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            let (rows, cols) = terminal_size();
            let theme = state.theme().clone();
            let glyphs = state.glyphs();
            let box_w = 48usize.min(cols.saturating_sub(4)).max(28);
            let box_h = 8 + lines.len();
            let top = rows.saturating_sub(box_h) / 2;
            let left = cols.saturating_sub(box_w) / 2;
            let mut buf = String::new();
            clear_buf(&mut buf, &theme);
            draw_box(
                &mut buf,
                top,
                left,
                box_h,
                box_w,
                title,
                &theme,
                Role::Title,
                glyphs,
            );
            for (i, line) in lines.iter().enumerate() {
                center(
                    &mut buf,
                    top + 2 + i,
                    &trim(line, box_w - 6),
                    &theme,
                    if i == 0 { Role::Accent } else { Role::Normal },
                    i == 0,
                    cols,
                );
            }
            for i in 0..if restart { 2 } else { 1 } {
                let text = button_text(glyphs, options[i], i == selected);
                let row = top + box_h - 3 + i;
                center(
                    &mut buf,
                    row,
                    &text,
                    &theme,
                    if i == selected {
                        Role::Highlight
                    } else {
                        Role::Muted
                    },
                    i == selected,
                    cols,
                );
            }
            flush(&buf);
            dirty = false;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up | Key::Down | Key::Char('w') | Key::Char('s') if restart => {
                    selected = 1 - selected;
                    dirty = true;
                }
                Key::Enter | Key::Space => {
                    click_effect(state, options[selected]);
                    return restart && selected == 0;
                }
                Key::Esc | Key::Char('q') => return false,
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(30));
    }
}

fn confirm_dialog(state: &mut AppState, title: &str, lines: &[&str]) -> bool {
    let mut selected = 1usize;
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            let (rows, cols) = terminal_size();
            let theme = state.theme().clone();
            let glyphs = state.glyphs();
            let box_w = 56usize.min(cols.saturating_sub(4)).max(32);
            let box_h = 9 + lines.len();
            let top = rows.saturating_sub(box_h) / 2;
            let left = cols.saturating_sub(box_w) / 2;
            let mut buf = String::new();
            clear_buf(&mut buf, &theme);
            draw_box(
                &mut buf,
                top,
                left,
                box_h,
                box_w,
                title,
                &theme,
                Role::Danger,
                glyphs,
            );
            for (i, line) in lines.iter().enumerate() {
                center(
                    &mut buf,
                    top + 2 + i,
                    line,
                    &theme,
                    Role::Normal,
                    false,
                    cols,
                );
            }
            let yes = button_text(glyphs, "YES, ERASE", selected == 0);
            let no = button_text(glyphs, "CANCEL", selected == 1);
            center(
                &mut buf,
                top + box_h - 4,
                &yes,
                &theme,
                if selected == 0 {
                    Role::Danger
                } else {
                    Role::Muted
                },
                selected == 0,
                cols,
            );
            center(
                &mut buf,
                top + box_h - 2,
                &no,
                &theme,
                if selected == 1 {
                    Role::Highlight
                } else {
                    Role::Muted
                },
                selected == 1,
                cols,
            );
            flush(&buf);
            dirty = false;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up
                | Key::Down
                | Key::Left
                | Key::Right
                | Key::Char('w')
                | Key::Char('s')
                | Key::Char('a')
                | Key::Char('d') => {
                    selected = 1 - selected;
                    dirty = true;
                }
                Key::Enter | Key::Space => {
                    click_effect(state, if selected == 0 { "erase" } else { "cancel" });
                    return selected == 0;
                }
                Key::Esc | Key::Char('q') => return false,
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(30));
    }
}

fn trim(text: &str, width: usize) -> String {
    if visible_width(text) <= width {
        text.to_string()
    } else if width <= 3 {
        ".".repeat(width)
    } else {
        let mut out: String = text.chars().take(width - 3).collect();
        out.push_str("...");
        out
    }
}

fn visible_width(text: &str) -> usize {
    text.chars().count()
}

fn pad_right(text: &str, width: usize) -> String {
    let mut out = text.to_string();
    let used = visible_width(text);
    if used < width {
        out.push_str(&" ".repeat(width - used));
    }
    out
}

fn repeat_glyph(glyph: &str, count: usize) -> String {
    let mut out = String::new();
    for _ in 0..count {
        out.push_str(glyph);
    }
    out
}

fn button_text(glyphs: &GlyphSet, label: &str, selected: bool) -> String {
    if selected {
        format!("{} {} {}", glyphs.button_left, label, glyphs.button_right)
    } else {
        format!("  {label}  ")
    }
}

fn draw_title_block(buf: &mut String, state: &AppState, row: usize, cols: usize) {
    let theme = state.theme();
    let glyphs = state.glyphs();
    if state.app_title == DEFAULT_TITLE && cols >= TITLE_ART[0].len() + 2 {
        for (i, line) in TITLE_ART.iter().enumerate() {
            center(buf, row + i, line, theme, Role::Title, true, cols);
        }
        return;
    }

    let title = trim(&state.app_title, cols.saturating_sub(10).max(10));
    let title_width = visible_width(&title);
    let rule_width = (title_width + 8).min(cols.saturating_sub(4)).max(12);
    let rule = repeat_glyph(glyphs.horizontal, rule_width);
    center(buf, row + 1, &rule, theme, Role::Muted, false, cols);
    center(
        buf,
        row + 2,
        &format!("{} {} {}", glyphs.title_left, title, glyphs.title_right),
        theme,
        Role::Title,
        true,
        cols,
    );
    center(buf, row + 3, &rule, theme, Role::Muted, false, cols);
}

fn home_menu(state: &mut AppState) -> io::Result<()> {
    let mut selected = 0usize;
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            draw_home(state, selected);
            dirty = false;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up | Key::Char('w') => {
                    selected = selected.saturating_add(2) % 3;
                    dirty = true;
                }
                Key::Down | Key::Char('s') => {
                    selected = (selected + 1) % 3;
                    dirty = true;
                }
                Key::Esc | Key::Char('q') => {
                    click_effect(state, "exit");
                    return Ok(());
                }
                Key::Enter | Key::Space => match selected {
                    0 => {
                        click_effect(state, "play");
                        play_menu(state);
                        dirty = true;
                    }
                    1 => {
                        click_effect(state, "settings");
                        settings_menu(state);
                        dirty = true;
                    }
                    _ => {
                        click_effect(state, "exit");
                        return Ok(());
                    }
                },
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn draw_home(state: &AppState, selected: usize) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let glyphs = state.glyphs();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    if rows < 22 || cols < 76 {
        center(
            &mut buf,
            rows / 2 - 2,
            "TUI Arcade",
            &theme,
            Role::Title,
            true,
            cols,
        );
        center(
            &mut buf,
            rows / 2,
            "Resize terminal to at least 76x22.",
            &theme,
            Role::Danger,
            false,
            cols,
        );
        flush(&buf);
        return;
    }
    draw_title_block(&mut buf, state, 1, cols);
    let content_w = cols.saturating_sub(6);
    let left = cols.saturating_sub(content_w) / 2;
    let top = 9;
    let panel_h = rows.saturating_sub(top + 3).max(11);
    let menu_w = (content_w / 3).clamp(28, 40);
    let status_gap = 4;
    let status_w = content_w.saturating_sub(menu_w + status_gap).max(34);
    draw_box(
        &mut buf,
        top,
        left,
        panel_h,
        menu_w,
        "MENU",
        &theme,
        Role::Accent,
        glyphs,
    );
    let options = ["Play", "Settings", "Exit"];
    for (i, option) in options.iter().enumerate() {
        let row = top + 2 + i * 2;
        let label = if i == selected {
            format!("{} {option}", glyphs.selector)
        } else {
            format!("  {option}")
        };
        if i == selected {
            put_inv(
                &mut buf,
                row,
                left + 4,
                &pad_right(&label, menu_w.saturating_sub(8)),
                &theme,
                Role::Highlight,
            );
        } else {
            put(
                &mut buf,
                row,
                left + 4,
                &label,
                &theme,
                Role::Secondary,
                false,
            );
        }
    }
    let panel_left = left + menu_w + status_gap;
    draw_box(
        &mut buf,
        top,
        panel_left,
        panel_h,
        status_w,
        "STATUS",
        &theme,
        Role::Accent,
        glyphs,
    );
    put(
        &mut buf,
        top + 2,
        panel_left + 2,
        &format!("Games: {}", GAMES.len()),
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + 3,
        panel_left + 2,
        &format!(
            "Difficulty: {}{}",
            state.difficulty().name,
            if state.endless_mode { " + Endless" } else { "" }
        ),
        &theme,
        Role::Secondary,
        false,
    );
    put(
        &mut buf,
        top + 4,
        panel_left + 2,
        &format!("Color theme: {}", state.theme().name),
        &theme,
        Role::Accent,
        false,
    );
    put(
        &mut buf,
        top + 5,
        panel_left + 2,
        &format!("Glyphs: {}", state.glyphs().name),
        &theme,
        Role::Secondary,
        false,
    );
    put(
        &mut buf,
        top + 6,
        panel_left + 2,
        &format!(
            "Pong: {} assist, {}",
            state.pong_assist_name(),
            state.pong_speed_name()
        ),
        &theme,
        Role::Normal,
        false,
    );
    put(
        &mut buf,
        top + 7,
        panel_left + 2,
        &format!("Sound: {}", if state.sound_enabled { "on" } else { "off" }),
        &theme,
        Role::Normal,
        false,
    );
    put(
        &mut buf,
        top + 8,
        panel_left + 2,
        &format!(
            "Click FX: {}",
            if state.click_effects { "on" } else { "off" }
        ),
        &theme,
        Role::Normal,
        false,
    );
    put(
        &mut buf,
        top + 9,
        panel_left + 2,
        &format!(
            "Saved scores: {}   Favorites: {}",
            state.scores.len(),
            state.favorites.len()
        ),
        &theme,
        Role::Success,
        false,
    );
    put(
        &mut buf,
        top + 11,
        panel_left + 2,
        "Enter selects. Q exits.",
        &theme,
        Role::Muted,
        false,
    );
    if panel_h > 13 {
        put(
            &mut buf,
            top + 12,
            panel_left + 2,
            &format!("Screen: {}x{} fullscreen-aware", cols, rows),
            &theme,
            Role::Muted,
            false,
        );
    }
    center(
        &mut buf,
        rows - 2,
        "Rust terminal arcade. Run it with the games command.",
        &theme,
        Role::Muted,
        true,
        cols,
    );
    flush(&buf);
}

fn game_category(game: &GameInfo) -> GameCategory {
    match game.kind {
        GameKind::Tetris
        | GameKind::Minefield
        | GameKind::Maze
        | GameKind::Memory
        | GameKind::Number
        | GameKind::Circuit
        | GameKind::BombSweeper
        | GameKind::PixelPop
        | GameKind::IceSlide
        | GameKind::SignalTrace
        | GameKind::TicTacToe => GameCategory::Puzzle,
        GameKind::Chess | GameKind::Checkers => GameCategory::Strategy,
        GameKind::Pong
        | GameKind::TronCycles
        | GameKind::Breakout
        | GameKind::Meteor
        | GameKind::Target
        | GameKind::Whack
        | GameKind::Simon
        | GameKind::Reaction
        | GameKind::Star
        | GameKind::BlockDrop
        | GameKind::CometCatcher
        | GameKind::CargoCatch
        | GameKind::GemRush
        | GameKind::DataStorm
        | GameKind::RainRunner
        | GameKind::ByteBlaster => GameCategory::Arcade,
        GameKind::Racer | GameKind::River | GameKind::FuelRun | GameKind::StormSurge => {
            GameCategory::Racing
        }
        GameKind::Dungeon
        | GameKind::Coin
        | GameKind::Frog
        | GameKind::PearlDiver
        | GameKind::VaultEscape
        | GameKind::CrystalCavern => GameCategory::Adventure,
        GameKind::Micro(index) => MICRO_GAMES
            .get(index)
            .map(|game| game.category)
            .unwrap_or(GameCategory::Arcade),
        GameKind::TronGridRun => GameCategory::Action,
        _ => GameCategory::Action,
    }
}

fn filtered_game_indices(state: &AppState, category: GameCategory, search: &str) -> Vec<usize> {
    let query = search.trim().to_ascii_lowercase();
    GAMES
        .iter()
        .enumerate()
        .filter(|(_, game)| {
            (category == GameCategory::All
                || (category == GameCategory::Favorites && state.favorites.contains(game.name))
                || game_category(game) == category)
                && (query.is_empty()
                    || game.name.to_ascii_lowercase().contains(&query)
                    || game.summary.to_ascii_lowercase().contains(&query)
                    || game_category(game)
                        .name()
                        .to_ascii_lowercase()
                        .contains(&query))
        })
        .map(|(index, _)| index)
        .collect()
}

fn play_menu(state: &mut AppState) {
    let mut selected = 0usize;
    let mut category_index = 0usize;
    let mut search = String::new();
    let mut search_mode = false;
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        let category = CATEGORIES[category_index];
        let filtered = filtered_game_indices(state, category, &search);
        if !filtered.is_empty() {
            selected = selected.min(filtered.len() - 1);
        } else {
            selected = 0;
        }
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            draw_play_menu(state, selected, category, &search, search_mode, &filtered);
            dirty = false;
        }
        if search_mode {
            if let Some(key) = read_text_key() {
                match key {
                    Key::Esc | Key::Enter => search_mode = false,
                    Key::Backspace => {
                        search.pop();
                    }
                    Key::Space if visible_width(&search) < 28 => search.push(' '),
                    Key::Char(ch) if !ch.is_control() && visible_width(&search) < 28 => {
                        search.push(ch);
                    }
                    _ => {}
                }
                dirty = true;
            }
            thread::sleep(Duration::from_millis(25));
            continue;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up | Key::Char('w') => {
                    if !filtered.is_empty() {
                        selected = (selected + filtered.len() - 1) % filtered.len();
                    }
                    dirty = true;
                }
                Key::Down | Key::Char('s') => {
                    if !filtered.is_empty() {
                        selected = (selected + 1) % filtered.len();
                    }
                    dirty = true;
                }
                Key::Left | Key::Char('a') => {
                    state.difficulty_index =
                        (state.difficulty_index + DIFFICULTIES.len() - 1) % DIFFICULTIES.len();
                    save_difficulty_index(state.difficulty_index);
                    dirty = true;
                }
                Key::Right | Key::Char('d') => {
                    state.difficulty_index = (state.difficulty_index + 1) % DIFFICULTIES.len();
                    save_difficulty_index(state.difficulty_index);
                    dirty = true;
                }
                Key::Char('t') => {
                    state.theme_index = (state.theme_index + 1) % state.themes.len();
                    save_theme_index(state.theme_index);
                    dirty = true;
                }
                Key::Char('g') => {
                    state.glyph_index = (state.glyph_index + 1) % state.glyph_sets.len();
                    save_glyph_index(state.glyph_index);
                    dirty = true;
                }
                Key::Char('c') => {
                    theme_lab(state);
                    dirty = true;
                }
                Key::Char('[') => {
                    category_index = (category_index + CATEGORIES.len() - 1) % CATEGORIES.len();
                    selected = 0;
                    dirty = true;
                }
                Key::Char(']') => {
                    category_index = (category_index + 1) % CATEGORIES.len();
                    selected = 0;
                    dirty = true;
                }
                Key::Char('/') => {
                    search_mode = true;
                    dirty = true;
                }
                Key::Char('x') if !search.is_empty() => {
                    search.clear();
                    selected = 0;
                    dirty = true;
                }
                Key::Char('f') => {
                    if let Some(&game_index) = filtered.get(selected) {
                        let name = GAMES[game_index].name;
                        toggle_favorite(state, name);
                        click_effect(state, "favorite");
                        dirty = true;
                    }
                }
                Key::Enter | Key::Space => {
                    if let Some(&game_index) = filtered.get(selected) {
                        let game = &GAMES[game_index];
                        click_effect(state, game.name);
                        play_game(state, game);
                        dirty = true;
                    }
                }
                Key::Esc | Key::Char('q') => {
                    click_effect(state, "home");
                    return;
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn draw_play_menu(
    state: &AppState,
    selected: usize,
    category: GameCategory,
    search: &str,
    search_mode: bool,
    filtered: &[usize],
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let glyphs = state.glyphs();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    if rows < 22 || cols < 76 {
        center(
            &mut buf,
            rows / 2 - 2,
            "TUI Arcade",
            &theme,
            Role::Title,
            true,
            cols,
        );
        center(
            &mut buf,
            rows / 2,
            "Resize terminal to at least 76x22.",
            &theme,
            Role::Danger,
            false,
            cols,
        );
        flush(&buf);
        return;
    }
    draw_title_block(&mut buf, state, 1, cols);
    let panel_top = 8;
    let panel_h = rows.saturating_sub(panel_top + 4).max(12);
    let content_w = cols.saturating_sub(6);
    let content_left = cols.saturating_sub(content_w) / 2;
    let list_w = (content_w * 45 / 100).clamp(36, 72);
    let list_left = content_left;
    let detail_left = list_left + list_w + 4;
    let detail_w = content_w.saturating_sub(list_w + 4).max(34);
    center(
        &mut buf,
        panel_top - 1,
        &trim(
            &format!(
                "Difficulty: < {}{} >   Category: {}   Theme: {}   {}",
                state.difficulty().name,
                if state.endless_mode { " Endless" } else { "" },
                category.name(),
                format!("{} / {}", state.theme().name, state.glyphs().name),
                state.difficulty().description
            ),
            cols - 4,
        ),
        &theme,
        Role::Accent,
        true,
        cols,
    );
    draw_box(
        &mut buf,
        panel_top,
        list_left,
        panel_h,
        list_w,
        "PLAY LIBRARY",
        &theme,
        Role::Accent,
        glyphs,
    );
    let visible = panel_h.saturating_sub(4).max(1);
    let start = selected
        .saturating_sub(visible - 1)
        .min(filtered.len().saturating_sub(visible));
    for (row_index, pos) in (start..(start + visible).min(filtered.len())).enumerate() {
        let index = filtered[pos];
        let game = &GAMES[index];
        let score = state.scores.get(game.name).copied().unwrap_or(0);
        let row = panel_top + 2 + row_index;
        let favorite_mark = if state.favorites.contains(game.name) {
            "*"
        } else {
            " "
        };
        let label = format!(
            "{}{} {:03}. {}",
            if pos == selected {
                glyphs.selector
            } else {
                " "
            },
            favorite_mark,
            index + 1,
            trim(game.name, list_w - 16)
        );
        if pos == selected {
            put_inv(
                &mut buf,
                row,
                list_left + 2,
                &pad_right(&label, list_w - 4),
                &theme,
                Role::Highlight,
            );
        } else {
            put(
                &mut buf,
                row,
                list_left + 2,
                &label,
                &theme,
                if pos % 2 == 0 {
                    Role::Secondary
                } else {
                    Role::Normal
                },
                false,
            );
        }
        if score > 0 {
            put(
                &mut buf,
                row,
                list_left + list_w - 8,
                &score.to_string(),
                &theme,
                Role::Success,
                false,
            );
        }
    }
    draw_scrollbar(
        &mut buf,
        panel_top + 1,
        list_left + list_w - 3,
        panel_h - 2,
        filtered.len().max(1),
        visible,
        start,
        &theme,
        Role::Muted,
        glyphs,
    );
    let Some(&selected_index) = filtered.get(selected) else {
        draw_box(
            &mut buf,
            panel_top,
            detail_left,
            panel_h,
            detail_w,
            "NO MATCHES",
            &theme,
            Role::Accent,
            glyphs,
        );
        put(
            &mut buf,
            panel_top + 2,
            detail_left + 2,
            "No games match this category/search.",
            &theme,
            Role::Danger,
            true,
        );
        put(
            &mut buf,
            panel_top + 4,
            detail_left + 2,
            "Press / to edit search, X to clear, [ or ] for categories.",
            &theme,
            Role::Muted,
            false,
        );
        center(
            &mut buf,
            rows - 2,
            "Search and categories filter the full arcade library.",
            &theme,
            Role::Muted,
            true,
            cols,
        );
        flush(&buf);
        return;
    };
    let game = &GAMES[selected_index];
    draw_box(
        &mut buf,
        panel_top,
        detail_left,
        panel_h,
        detail_w,
        &game.name.to_ascii_uppercase(),
        &theme,
        Role::Accent,
        glyphs,
    );
    let wrapped = wrap(game.summary, detail_w - 5);
    for (i, line) in wrapped.iter().take(4).enumerate() {
        put(
            &mut buf,
            panel_top + 2 + i,
            detail_left + 2,
            line,
            &theme,
            Role::Normal,
            false,
        );
    }
    put(
        &mut buf,
        panel_top + 6,
        detail_left + 2,
        &format!(
            "{}   High score: {}   Favorite: {}",
            game_category(game).name(),
            state.scores.get(game.name).copied().unwrap_or(0),
            if state.favorites.contains(game.name) {
                "yes"
            } else {
                "no"
            }
        ),
        &theme,
        Role::Success,
        true,
    );
    if panel_h <= 12 {
        let controls_top = panel_top + 7;
        put(
            &mut buf,
            controls_top,
            detail_left + 2,
            "W/S choose | A/D difficulty | / search",
            &theme,
            Role::Muted,
            false,
        );
        put(
            &mut buf,
            controls_top + 1,
            detail_left + 2,
            "[ ] category | F favorite | T colors | G glyphs",
            &theme,
            Role::Muted,
            false,
        );
        put(
            &mut buf,
            controls_top + 2,
            detail_left + 2,
            "Enter launch | P pause | Q home",
            &theme,
            Role::Muted,
            false,
        );
    } else {
        let controls_top = panel_top + panel_h.saturating_sub(6);
        put(
            &mut buf,
            controls_top,
            detail_left + 2,
            "Controls",
            &theme,
            Role::Secondary,
            true,
        );
        put(
            &mut buf,
            controls_top + 1,
            detail_left + 2,
            &format!(
                "Library: {}/{} shown   Search: {}{}",
                filtered.len(),
                GAMES.len(),
                if search_mode { "editing " } else { "" },
                if search.is_empty() { "(empty)" } else { search }
            ),
            &theme,
            Role::Muted,
            false,
        );
        put(
            &mut buf,
            controls_top + 2,
            detail_left + 2,
            "/ search | X clear | [ ] category | F favorite",
            &theme,
            Role::Muted,
            false,
        );
        put(
            &mut buf,
            controls_top + 3,
            detail_left + 2,
            "Left/Right: difficulty | T colors | G glyphs | C lab",
            &theme,
            Role::Muted,
            false,
        );
        put(
            &mut buf,
            controls_top + 4,
            detail_left + 2,
            "Enter launch | P pause | Q home",
            &theme,
            Role::Muted,
            false,
        );
    }
    center(
        &mut buf,
        rows - 2,
        "Scores save automatically. Rust build keeps the terminal fast.",
        &theme,
        Role::Muted,
        true,
        cols,
    );
    flush(&buf);
}

fn wrap(text: &str, width: usize) -> Vec<String> {
    let mut lines = Vec::new();
    let mut line = String::new();
    for word in text.split_whitespace() {
        let candidate = if line.is_empty() {
            word.to_string()
        } else {
            format!("{line} {word}")
        };
        if visible_width(&candidate) > width && !line.is_empty() {
            lines.push(line);
            line = word.to_string();
        } else {
            line = candidate;
        }
    }
    if !line.is_empty() {
        lines.push(line);
    }
    lines
}

const SETTING_DIFFICULTY: usize = 0;
const SETTING_ENDLESS: usize = 1;
const SETTING_PONG_ASSIST: usize = 2;
const SETTING_PONG_SPEED: usize = 3;
const SETTING_COLOR_THEME: usize = 4;
const SETTING_GLYPH_SET: usize = 5;
const SETTING_STARTUP_TITLE: usize = 6;
const SETTING_TEXT_COLOR: usize = 7;
const SETTING_TITLE_COLOR: usize = 8;
const SETTING_ACCENT_COLOR: usize = 9;
const SETTING_SECONDARY_COLOR: usize = 10;
const SETTING_DANGER_COLOR: usize = 11;
const SETTING_SUCCESS_COLOR: usize = 12;
const SETTING_MUTED_COLOR: usize = 13;
const SETTING_HIGHLIGHT_COLOR: usize = 14;
const SETTING_BACKGROUND: usize = 15;
const SETTING_SOUND: usize = 16;
const SETTING_SOUND_TEST: usize = 17;
const SETTING_CLICK_EFFECTS: usize = 18;
const SETTING_CONTROLS: usize = 19;
const SETTING_ERASE_SCORES: usize = 20;
const SETTING_BACK: usize = 21;

fn settings_rows(state: &AppState) -> Vec<(String, String)> {
    vec![
        (
            "Difficulty".to_string(),
            state.difficulty().name.to_string(),
        ),
        (
            "Endless mode".to_string(),
            if state.endless_mode { "on" } else { "off" }.to_string(),
        ),
        (
            "Pong assist".to_string(),
            state.pong_assist_name().to_string(),
        ),
        (
            "Pong speed".to_string(),
            state.pong_speed_name().to_string(),
        ),
        ("Color theme".to_string(), state.theme().name.clone()),
        (
            "Glyph set".to_string(),
            format!("{} - {}", state.glyphs().name, state.glyphs().description),
        ),
        ("Startup title".to_string(), state.app_title.clone()),
        (
            "Text color".to_string(),
            COLOR_NAMES[state.theme().fg as usize].to_string(),
        ),
        (
            "Title color".to_string(),
            COLOR_NAMES[state.theme().title as usize].to_string(),
        ),
        (
            "Accent color".to_string(),
            COLOR_NAMES[state.theme().accent as usize].to_string(),
        ),
        (
            "Secondary color".to_string(),
            COLOR_NAMES[state.theme().secondary as usize].to_string(),
        ),
        (
            "Danger color".to_string(),
            COLOR_NAMES[state.theme().danger as usize].to_string(),
        ),
        (
            "Success color".to_string(),
            COLOR_NAMES[state.theme().success as usize].to_string(),
        ),
        (
            "Muted color".to_string(),
            COLOR_NAMES[state.theme().muted as usize].to_string(),
        ),
        (
            "Highlight color".to_string(),
            COLOR_NAMES[state.theme().highlight as usize].to_string(),
        ),
        (
            "Background".to_string(),
            state
                .theme()
                .bg
                .map(|c| COLOR_NAMES[c as usize].to_string())
                .unwrap_or_else(|| "Terminal default".to_string()),
        ),
        (
            "Sound".to_string(),
            if state.sound_enabled { "on" } else { "off" }.to_string(),
        ),
        ("Sound test".to_string(), "play chime".to_string()),
        (
            "Click effects".to_string(),
            if state.click_effects { "on" } else { "off" }.to_string(),
        ),
        (
            "Controls".to_string(),
            format!(
                "{}/{}/{}/{} action {}",
                control_label(state.controls.up),
                control_label(state.controls.down),
                control_label(state.controls.left),
                control_label(state.controls.right),
                control_label(state.controls.action)
            ),
        ),
        (
            "Erase scores".to_string(),
            format!("{} saved", state.scores.len()),
        ),
        ("Back".to_string(), "return home".to_string()),
    ]
}

fn settings_menu(state: &mut AppState) {
    let mut selected = 0usize;
    let mut message = "Color themes and glyph sets mix independently.".to_string();
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        let rows = settings_rows(state);
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            draw_settings(state, selected, &message, &rows);
            dirty = false;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up | Key::Char('w') => selected = (selected + rows.len() - 1) % rows.len(),
                Key::Down | Key::Char('s') => selected = (selected + 1) % rows.len(),
                Key::Left | Key::Char('a') => {
                    settings_adjust(state, selected, -1, &mut message);
                }
                Key::Right | Key::Char('d') => {
                    settings_adjust(state, selected, 1, &mut message);
                }
                Key::Enter | Key::Space => match selected {
                    SETTING_DIFFICULTY | SETTING_ENDLESS | SETTING_PONG_ASSIST
                    | SETTING_PONG_SPEED | SETTING_COLOR_THEME | SETTING_GLYPH_SET => {
                        settings_adjust(state, selected, 1, &mut message)
                    }
                    SETTING_STARTUP_TITLE => {
                        message = if edit_title_screen(state) {
                            "Startup title saved.".to_string()
                        } else {
                            "Title edit cancelled.".to_string()
                        };
                    }
                    SETTING_TEXT_COLOR
                    | SETTING_TITLE_COLOR
                    | SETTING_ACCENT_COLOR
                    | SETTING_SECONDARY_COLOR
                    | SETTING_DANGER_COLOR
                    | SETTING_SUCCESS_COLOR
                    | SETTING_MUTED_COLOR
                    | SETTING_HIGHLIGHT_COLOR
                    | SETTING_BACKGROUND => settings_adjust(state, selected, 1, &mut message),
                    SETTING_SOUND => {
                        state.sound_enabled = !state.sound_enabled;
                        save_sound_enabled(state.sound_enabled);
                        click_effect(state, "sound");
                        message = format!(
                            "Sound {}.",
                            if state.sound_enabled {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );
                    }
                    SETTING_SOUND_TEST => {
                        play_sound(state, "score");
                        message = "Played a macOS sound test.".to_string();
                    }
                    SETTING_CLICK_EFFECTS => {
                        state.click_effects = !state.click_effects;
                        save_click_effects(state.click_effects);
                        click_effect(state, "click fx");
                        message = format!(
                            "Click effects {}.",
                            if state.click_effects {
                                "enabled"
                            } else {
                                "disabled"
                            }
                        );
                    }
                    SETTING_CONTROLS => {
                        controls_menu(state);
                        message = "Controls updated.".to_string();
                    }
                    SETTING_ERASE_SCORES => {
                        if confirm_dialog(
                            state,
                            "ERASE SCORES?",
                            &[
                                "This permanently deletes saved high scores.",
                                "Your custom theme is not affected.",
                                "Choose YES only if you mean it.",
                            ],
                        ) {
                            message = if erase_scores(state) {
                                "Scores erased.".to_string()
                            } else {
                                "Could not erase scores.".to_string()
                            };
                        } else {
                            message = "Score erase cancelled.".to_string();
                        }
                    }
                    SETTING_BACK => {
                        click_effect(state, "back");
                        return;
                    }
                    _ => {}
                },
                Key::Esc | Key::Char('q') => {
                    click_effect(state, "back");
                    return;
                }
                _ => {}
            }
            dirty = true;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn settings_adjust(state: &mut AppState, selected: usize, delta: i32, message: &mut String) {
    match selected {
        SETTING_DIFFICULTY => {
            state.difficulty_index = wrap_index(state.difficulty_index, DIFFICULTIES.len(), delta);
            save_difficulty_index(state.difficulty_index);
            *message = format!("Difficulty set to {}.", state.difficulty().name);
        }
        SETTING_ENDLESS => {
            state.endless_mode = !state.endless_mode;
            save_endless_mode(state.endless_mode);
            *message = format!(
                "Endless mode {}.",
                if state.endless_mode {
                    "enabled"
                } else {
                    "disabled"
                }
            );
        }
        SETTING_PONG_ASSIST => {
            state.pong_assist_index =
                wrap_index(state.pong_assist_index, PONG_ASSIST_NAMES.len(), delta);
            save_pong_options(state.pong_assist_index, state.pong_speed_index);
            *message = format!("Pong assist set to {}.", state.pong_assist_name());
        }
        SETTING_PONG_SPEED => {
            state.pong_speed_index =
                wrap_index(state.pong_speed_index, PONG_SPEED_NAMES.len(), delta);
            save_pong_options(state.pong_assist_index, state.pong_speed_index);
            *message = format!("Pong speed set to {}.", state.pong_speed_name());
        }
        SETTING_COLOR_THEME => {
            state.theme_index = wrap_index(state.theme_index, state.themes.len(), delta);
            save_theme_index(state.theme_index);
            *message = "Color theme changed.".to_string();
        }
        SETTING_GLYPH_SET => {
            state.glyph_index = wrap_index(state.glyph_index, state.glyph_sets.len(), delta);
            save_glyph_index(state.glyph_index);
            *message = "Glyph set changed independently of colors.".to_string();
        }
        SETTING_STARTUP_TITLE => {
            *message = "Press Enter to edit the startup title.".to_string();
        }
        SETTING_TEXT_COLOR => {
            let theme = state.custom_theme_mut();
            theme.fg = wrap_color(theme.fg, delta);
            save_custom_theme(theme);
            *message = "Custom text color saved.".to_string();
        }
        SETTING_TITLE_COLOR => {
            let theme = state.custom_theme_mut();
            theme.title = wrap_color(theme.title, delta);
            save_custom_theme(theme);
            *message = "Custom title color saved.".to_string();
        }
        SETTING_ACCENT_COLOR => {
            let theme = state.custom_theme_mut();
            theme.accent = wrap_color(theme.accent, delta);
            save_custom_theme(theme);
            *message = "Custom accent color saved.".to_string();
        }
        SETTING_SECONDARY_COLOR => {
            let theme = state.custom_theme_mut();
            theme.secondary = wrap_color(theme.secondary, delta);
            save_custom_theme(theme);
            *message = "Custom secondary color saved.".to_string();
        }
        SETTING_DANGER_COLOR => {
            let theme = state.custom_theme_mut();
            theme.danger = wrap_color(theme.danger, delta);
            save_custom_theme(theme);
            *message = "Custom danger color saved.".to_string();
        }
        SETTING_SUCCESS_COLOR => {
            let theme = state.custom_theme_mut();
            theme.success = wrap_color(theme.success, delta);
            save_custom_theme(theme);
            *message = "Custom success color saved.".to_string();
        }
        SETTING_MUTED_COLOR => {
            let theme = state.custom_theme_mut();
            theme.muted = wrap_color(theme.muted, delta);
            save_custom_theme(theme);
            *message = "Custom muted color saved.".to_string();
        }
        SETTING_HIGHLIGHT_COLOR => {
            let theme = state.custom_theme_mut();
            theme.highlight = wrap_color(theme.highlight, delta);
            save_custom_theme(theme);
            *message = "Custom highlight color saved.".to_string();
        }
        SETTING_BACKGROUND => {
            let theme = state.custom_theme_mut();
            let next = match theme.bg {
                None if delta >= 0 => Some(0),
                None => Some(15),
                Some(color) => {
                    let value = color as i32 + delta;
                    if value < 0 || value > 15 {
                        None
                    } else {
                        Some(value as u8)
                    }
                }
            };
            theme.bg = next;
            save_custom_theme(theme);
            *message = "Custom background saved.".to_string();
        }
        SETTING_CONTROLS => {
            *message = "Press Enter to edit controls.".to_string();
        }
        _ => {}
    }
}

fn controls_menu(state: &mut AppState) {
    let mut selected = 0usize;
    let mut message = "Enter changes a binding. Reset restores WASD / Space / P / Q.".to_string();
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            draw_controls_menu(state, selected, &message);
            dirty = false;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up | Key::Char('w') => {
                    selected = (selected + 8) % 9;
                    dirty = true;
                }
                Key::Down | Key::Char('s') => {
                    selected = (selected + 1) % 9;
                    dirty = true;
                }
                Key::Enter | Key::Space => {
                    if selected < 7 {
                        let label = control_row_label(selected);
                        if let Some(ch) = capture_control_key(state, label) {
                            if control_taken(state.controls, selected, ch) {
                                message = format!("{} is already bound.", control_label(ch));
                            } else {
                                set_control_value(&mut state.controls, selected, ch);
                                save_controls(state.controls);
                                sync_controls(state.controls);
                                message = format!("{label} set to {}.", control_label(ch));
                            }
                        } else {
                            message = "Control edit cancelled.".to_string();
                        }
                    } else if selected == 7 {
                        state.controls = Controls::default();
                        save_controls(state.controls);
                        sync_controls(state.controls);
                        message = "Controls reset.".to_string();
                    } else {
                        return;
                    }
                    dirty = true;
                }
                Key::Esc | Key::Char('q') => return,
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn control_row_label(index: usize) -> &'static str {
    match index {
        0 => "Move up",
        1 => "Move down",
        2 => "Move left",
        3 => "Move right",
        4 => "Action",
        5 => "Pause",
        6 => "Quit",
        7 => "Reset defaults",
        _ => "Back",
    }
}

fn control_value(controls: Controls, index: usize) -> char {
    match index {
        0 => controls.up,
        1 => controls.down,
        2 => controls.left,
        3 => controls.right,
        4 => controls.action,
        5 => controls.pause,
        6 => controls.quit,
        _ => '\0',
    }
}

fn set_control_value(controls: &mut Controls, index: usize, value: char) {
    match index {
        0 => controls.up = value,
        1 => controls.down = value,
        2 => controls.left = value,
        3 => controls.right = value,
        4 => controls.action = value,
        5 => controls.pause = value,
        6 => controls.quit = value,
        _ => {}
    }
}

fn control_taken(controls: Controls, selected: usize, value: char) -> bool {
    (0..7).any(|index| index != selected && control_value(controls, index) == value)
}

fn draw_controls_menu(state: &AppState, selected: usize, message: &str) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let glyphs = state.glyphs();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    if rows < 22 || cols < 74 {
        center(
            &mut buf,
            rows / 2 - 2,
            "CONTROLS",
            &theme,
            Role::Title,
            true,
            cols,
        );
        center(
            &mut buf,
            rows / 2,
            "Resize terminal to at least 74x22.",
            &theme,
            Role::Danger,
            false,
            cols,
        );
        flush(&buf);
        return;
    }
    center(&mut buf, 1, "CONTROLS", &theme, Role::Title, true, cols);
    let box_w = 60usize.min(cols.saturating_sub(6)).max(44);
    let box_h = 17usize;
    let top = rows.saturating_sub(box_h) / 2;
    let left = cols.saturating_sub(box_w) / 2;
    draw_box(
        &mut buf,
        top,
        left,
        box_h,
        box_w,
        "REBIND",
        &theme,
        Role::Accent,
        glyphs,
    );
    for index in 0..9 {
        let row = top + 2 + index;
        let label = control_row_label(index);
        let value = if index < 7 {
            control_label(control_value(state.controls, index))
        } else if index == 7 {
            "restore".to_string()
        } else {
            "return".to_string()
        };
        let text = format!(
            "{} {:<18} {}",
            if selected == index {
                glyphs.selector
            } else {
                " "
            },
            label,
            value
        );
        if selected == index {
            put_inv(
                &mut buf,
                row,
                left + 3,
                &pad_right(&text, box_w - 6),
                &theme,
                Role::Highlight,
            );
        } else {
            put(
                &mut buf,
                row,
                left + 3,
                &text,
                &theme,
                if index < 7 {
                    Role::Normal
                } else {
                    Role::Secondary
                },
                false,
            );
        }
    }
    put(
        &mut buf,
        top + box_h - 4,
        left + 3,
        &trim(message, box_w - 6),
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + box_h - 2,
        left + 3,
        "Arrow keys and Enter/Esc always work as safety controls.",
        &theme,
        Role::Muted,
        false,
    );
    flush(&buf);
}

fn capture_control_key(state: &AppState, label: &str) -> Option<char> {
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            let (rows, cols) = terminal_size();
            let theme = state.theme().clone();
            let glyphs = state.glyphs();
            let box_w = 56usize.min(cols.saturating_sub(4)).max(34);
            let box_h = 9usize;
            let top = rows.saturating_sub(box_h) / 2;
            let left = cols.saturating_sub(box_w) / 2;
            let mut buf = String::new();
            clear_buf(&mut buf, &theme);
            draw_box(
                &mut buf,
                top,
                left,
                box_h,
                box_w,
                "PRESS KEY",
                &theme,
                Role::Accent,
                glyphs,
            );
            center(
                &mut buf,
                top + 2,
                &format!("Set {label}"),
                &theme,
                Role::Title,
                true,
                cols,
            );
            center(
                &mut buf,
                top + 4,
                "Press a letter, number, punctuation key, or Space.",
                &theme,
                Role::Normal,
                false,
                cols,
            );
            center(
                &mut buf,
                top + 6,
                "Esc cancels.",
                &theme,
                Role::Muted,
                false,
                cols,
            );
            flush(&buf);
            dirty = false;
        }
        if let Some(key) = read_text_key() {
            match key {
                Key::Esc => return None,
                Key::Space => return Some(' '),
                Key::Char(ch) if !ch.is_control() => return Some(ch.to_ascii_lowercase()),
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(20));
    }
}

fn edit_title_screen(state: &mut AppState) -> bool {
    let current_title = state.app_title.clone();
    let mut input = String::new();
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            let (rows, cols) = terminal_size();
            let theme = state.theme().clone();
            let glyphs = state.glyphs();
            let box_w = 66usize.min(cols.saturating_sub(4)).max(34);
            let box_h = 11usize;
            let top = rows.saturating_sub(box_h) / 2;
            let left = cols.saturating_sub(box_w) / 2;
            let field_w = box_w.saturating_sub(6).max(12);
            let mut shown = trim(&input, field_w.saturating_sub(1));
            shown.push('_');
            let mut buf = String::new();
            clear_buf(&mut buf, &theme);
            draw_box(
                &mut buf,
                top,
                left,
                box_h,
                box_w,
                "STARTUP TITLE",
                &theme,
                Role::Title,
                glyphs,
            );
            center(
                &mut buf,
                top + 2,
                &format!(
                    "Current: {}",
                    trim(&current_title, box_w.saturating_sub(16))
                ),
                &theme,
                Role::Muted,
                false,
                cols,
            );
            center(
                &mut buf,
                top + 3,
                "Type a replacement for the opening screens.",
                &theme,
                Role::Normal,
                false,
                cols,
            );
            put_inv(
                &mut buf,
                top + 5,
                left + 3,
                &pad_right(&shown, field_w),
                &theme,
                Role::Highlight,
            );
            center(
                &mut buf,
                top + 7,
                "Enter saves. Esc cancels. Backspace deletes.",
                &theme,
                Role::Muted,
                false,
                cols,
            );
            center(
                &mut buf,
                top + 8,
                "Saving an empty title restores the default.",
                &theme,
                Role::Muted,
                false,
                cols,
            );
            flush(&buf);
            dirty = false;
        }
        if let Some(key) = read_text_key() {
            match key {
                Key::Enter => {
                    let trimmed = input.trim();
                    state.app_title = if trimmed.is_empty() {
                        DEFAULT_TITLE.to_string()
                    } else {
                        trimmed.to_string()
                    };
                    save_app_title(&state.app_title);
                    click_effect(state, "save title");
                    return true;
                }
                Key::Esc => return false,
                Key::Backspace => {
                    input.pop();
                    dirty = true;
                }
                Key::Space if visible_width(&input) < MAX_TITLE_LEN => {
                    input.push(' ');
                    dirty = true;
                }
                Key::Char(ch) if !ch.is_control() && visible_width(&input) < MAX_TITLE_LEN => {
                    input.push(ch);
                    dirty = true;
                }
                _ => {}
            }
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn wrap_color(color: u8, delta: i32) -> u8 {
    ((color as i32 + delta).rem_euclid(16)) as u8
}

fn wrap_index(index: usize, len: usize, delta: i32) -> usize {
    ((index as i32 + delta).rem_euclid(len as i32)) as usize
}

fn draw_settings(state: &AppState, selected: usize, message: &str, rows_data: &[(String, String)]) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let glyphs = state.glyphs();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    if rows < 22 || cols < 76 {
        center(
            &mut buf,
            rows / 2 - 2,
            "Settings",
            &theme,
            Role::Title,
            true,
            cols,
        );
        center(
            &mut buf,
            rows / 2,
            "Resize terminal to at least 76x22.",
            &theme,
            Role::Danger,
            false,
            cols,
        );
        flush(&buf);
        return;
    }
    center(&mut buf, 1, "SETTINGS", &theme, Role::Title, true, cols);
    let content_w = cols.saturating_sub(6);
    let left = cols.saturating_sub(content_w) / 2;
    let top = 4;
    let box_w = content_w;
    let box_h = rows.saturating_sub(top + 4).max(18);
    draw_box(
        &mut buf,
        top,
        left,
        box_h,
        box_w,
        "ARCADE SETTINGS",
        &theme,
        Role::Accent,
        glyphs,
    );
    let visible = box_h.saturating_sub(6).max(1);
    let start = selected
        .saturating_sub(visible - 1)
        .min(rows_data.len().saturating_sub(visible));
    for (row_offset, i) in (start..(start + visible).min(rows_data.len())).enumerate() {
        let (label, value) = &rows_data[i];
        let row = top + 2 + row_offset;
        let marker = if i == selected { glyphs.selector } else { " " };
        let left_text = format!("{marker} {label}");
        if i == selected {
            put_inv(
                &mut buf,
                row,
                left + 3,
                &pad_right(&left_text, 27),
                &theme,
                Role::Highlight,
            );
            put_inv(
                &mut buf,
                row,
                left + 31,
                &pad_right(&trim(value, 28), 28),
                &theme,
                Role::Highlight,
            );
        } else {
            put(
                &mut buf,
                row,
                left + 3,
                &left_text,
                &theme,
                if i % 2 == 0 {
                    Role::Secondary
                } else {
                    Role::Normal
                },
                false,
            );
            put(
                &mut buf,
                row,
                left + 31,
                &trim(value, 28),
                &theme,
                Role::Muted,
                false,
            );
        }
    }
    draw_scrollbar(
        &mut buf,
        top + 1,
        left + box_w.saturating_sub(3),
        box_h.saturating_sub(5),
        rows_data.len().max(1),
        visible,
        start,
        &theme,
        Role::Muted,
        glyphs,
    );
    put(
        &mut buf,
        top + box_h.saturating_sub(3),
        left + 3,
        "Left/Right changes values. Enter activates rows.",
        &theme,
        Role::Muted,
        false,
    );
    put(
        &mut buf,
        top + box_h.saturating_sub(2),
        left + 3,
        &trim(message, box_w - 6),
        &theme,
        Role::Success,
        true,
    );
    center(
        &mut buf,
        rows - 2,
        "Esc/Q returns home.",
        &theme,
        Role::Muted,
        true,
        cols,
    );
    flush(&buf);
}

fn theme_lab(state: &mut AppState) {
    let mut selected = 0usize;
    let mut message =
        "Left/Right edits. 1/2/3 save slots. P copies preset. R randomizes.".to_string();
    let mut dirty = true;
    let mut last_size = terminal_size();
    loop {
        if size_changed(&mut last_size) {
            dirty = true;
        }
        if dirty {
            let theme = state.theme().clone();
            let glyphs = state.glyphs();
            let (rows, cols) = terminal_size();
            let mut buf = String::new();
            clear_buf(&mut buf, &theme);
            if rows < 24 || cols < 80 {
                center(
                    &mut buf,
                    rows / 2 - 2,
                    "Theme Lab",
                    &theme,
                    Role::Title,
                    true,
                    cols,
                );
                center(
                    &mut buf,
                    rows / 2,
                    "Resize terminal to at least 80x24.",
                    &theme,
                    Role::Danger,
                    false,
                    cols,
                );
                flush(&buf);
            } else {
                center(&mut buf, 1, "THEME LAB", &theme, Role::Title, true, cols);
                let left = cols / 2 - 38;
                draw_box(
                    &mut buf,
                    4,
                    left,
                    15,
                    76,
                    "CUSTOM COLOR ROLES",
                    &theme,
                    Role::Accent,
                    glyphs,
                );
                let roles = theme_role_rows(&theme);
                for (i, (name, value)) in roles.iter().enumerate() {
                    let row = 6 + i;
                    if i == selected {
                        put_inv(
                            &mut buf,
                            row,
                            left + 3,
                            &pad_right(
                                &format!("{} {:<18} {:<14}", glyphs.selector, name, value),
                                35,
                            ),
                            &theme,
                            Role::Highlight,
                        );
                    } else {
                        put(
                            &mut buf,
                            row,
                            left + 3,
                            &format!("  {:<18} {:<14}", name, value),
                            &theme,
                            Role::Normal,
                            false,
                        );
                    }
                }
                draw_box(
                    &mut buf,
                    6,
                    left + 42,
                    8,
                    30,
                    "PREVIEW",
                    &theme,
                    Role::Accent,
                    glyphs,
                );
                put(
                    &mut buf,
                    8,
                    left + 45,
                    "TUI Arcade",
                    &theme,
                    Role::Title,
                    true,
                );
                put(
                    &mut buf,
                    9,
                    left + 45,
                    &format!("{} Space Invaders", glyphs.selector),
                    &theme,
                    Role::Highlight,
                    true,
                );
                put(
                    &mut buf,
                    10,
                    left + 45,
                    "Player /A\\",
                    &theme,
                    Role::Accent,
                    true,
                );
                put(
                    &mut buf,
                    11,
                    left + 45,
                    "Enemy <M>",
                    &theme,
                    Role::Danger,
                    true,
                );
                put(
                    &mut buf,
                    12,
                    left + 45,
                    "Score 1200",
                    &theme,
                    Role::Success,
                    true,
                );
                put(
                    &mut buf,
                    16,
                    left + 3,
                    &message,
                    &theme,
                    Role::Success,
                    true,
                );
                center(&mut buf, rows - 2, "Up/Down role | Left/Right color | 1/2/3 save slots | 0 bg default | P copy preset | R random | Q back", &theme, Role::Muted, true, cols);
            }
            flush(&buf);
            dirty = false;
        }
        if let Some(key) = read_key() {
            match key {
                Key::Up | Key::Char('w') => selected = (selected + 8) % 9,
                Key::Down | Key::Char('s') => selected = (selected + 1) % 9,
                Key::Left | Key::Char('a') => {
                    theme_role_adjust(state, selected, -1);
                    message = "Custom color saved.".to_string();
                }
                Key::Right | Key::Char('d') => {
                    theme_role_adjust(state, selected, 1);
                    message = "Custom color saved.".to_string();
                }
                Key::Char('0') if selected == 1 => {
                    let theme = state.custom_theme_mut();
                    theme.bg = None;
                    save_custom_theme(theme);
                    message = "Custom background set to terminal default.".to_string();
                }
                Key::Char('p') => {
                    let current = state.theme().clone();
                    let custom = state.custom_theme_mut();
                    *custom = current;
                    custom.name = "Custom".to_string();
                    save_custom_theme(custom);
                    message = "Copied current preset into Custom.".to_string();
                }
                Key::Char('r') => {
                    let colors = [
                        state.rng.usize(16) as u8,
                        state.rng.usize(16) as u8,
                        state.rng.usize(16) as u8,
                        state.rng.usize(16) as u8,
                        state.rng.usize(16) as u8,
                    ];
                    let theme = state.custom_theme_mut();
                    theme.title = colors[0];
                    theme.accent = colors[1];
                    theme.secondary = colors[2];
                    theme.danger = colors[3];
                    theme.success = colors[4];
                    save_custom_theme(theme);
                    message = "Randomized Custom.".to_string();
                }
                Key::Char('1') | Key::Char('2') | Key::Char('3') => {
                    let slot = match key {
                        Key::Char('1') => 0,
                        Key::Char('2') => 1,
                        _ => 2,
                    };
                    message = if save_theme_slot(state, slot) {
                        format!("Saved current color set to Saved {}.", slot + 1)
                    } else {
                        "Could not save theme slot.".to_string()
                    };
                }
                Key::Esc | Key::Char('q') => return,
                _ => {}
            }
            dirty = true;
        }
        thread::sleep(Duration::from_millis(25));
    }
}

fn theme_role_rows(theme: &Theme) -> Vec<(String, String)> {
    vec![
        (
            "Text".to_string(),
            COLOR_NAMES[theme.fg as usize].to_string(),
        ),
        (
            "Background".to_string(),
            theme
                .bg
                .map(|c| COLOR_NAMES[c as usize].to_string())
                .unwrap_or_else(|| "Terminal default".to_string()),
        ),
        (
            "Title".to_string(),
            COLOR_NAMES[theme.title as usize].to_string(),
        ),
        (
            "Accent".to_string(),
            COLOR_NAMES[theme.accent as usize].to_string(),
        ),
        (
            "Secondary".to_string(),
            COLOR_NAMES[theme.secondary as usize].to_string(),
        ),
        (
            "Danger".to_string(),
            COLOR_NAMES[theme.danger as usize].to_string(),
        ),
        (
            "Success".to_string(),
            COLOR_NAMES[theme.success as usize].to_string(),
        ),
        (
            "Muted".to_string(),
            COLOR_NAMES[theme.muted as usize].to_string(),
        ),
        (
            "Highlight".to_string(),
            COLOR_NAMES[theme.highlight as usize].to_string(),
        ),
    ]
}

fn theme_role_adjust(state: &mut AppState, selected: usize, delta: i32) {
    let theme = state.custom_theme_mut();
    match selected {
        0 => theme.fg = wrap_color(theme.fg, delta),
        1 => {
            theme.bg = match theme.bg {
                None if delta >= 0 => Some(0),
                None => Some(15),
                Some(color) => {
                    let value = color as i32 + delta;
                    if value < 0 || value > 15 {
                        None
                    } else {
                        Some(value as u8)
                    }
                }
            }
        }
        2 => theme.title = wrap_color(theme.title, delta),
        3 => theme.accent = wrap_color(theme.accent, delta),
        4 => theme.secondary = wrap_color(theme.secondary, delta),
        5 => theme.danger = wrap_color(theme.danger, delta),
        6 => theme.success = wrap_color(theme.success, delta),
        7 => theme.muted = wrap_color(theme.muted, delta),
        8 => theme.highlight = wrap_color(theme.highlight, delta),
        _ => {}
    }
    save_custom_theme(theme);
}

fn play_game(state: &mut AppState, game: &GameInfo) {
    match game.kind {
        GameKind::Snake => game_snake(state),
        GameKind::Tetris => game_tetris(state),
        GameKind::Pong => game_pong(state),
        GameKind::TronCycles => game_tron_cycles(state),
        GameKind::TronGridRun => game_tron_grid_run(state),
        GameKind::Invaders => game_invaders(state),
        GameKind::Missile => game_missile(state),
        GameKind::Breakout => game_breakout(state),
        GameKind::Meteor => game_meteor(state),
        GameKind::Racer => game_racer(state),
        GameKind::Frog => game_frog(state),
        GameKind::Target => game_target(state, "Target Practice", false),
        GameKind::Coin => game_coin(state),
        GameKind::Minefield => game_minefield(state),
        GameKind::Maze => game_maze(state),
        GameKind::Whack => game_target(state, "Whack-a-Mole", true),
        GameKind::Simon => game_simon(state),
        GameKind::Reaction => game_reaction(state),
        GameKind::Flappy => game_flappy(state),
        GameKind::Asteroid => {
            game_side_scroll(state, "Asteroid Belt", "/A\\", "*", None, "WASD move")
        }
        GameKind::Star => game_star(state),
        GameKind::Laser => game_laser(state),
        GameKind::Dungeon => game_dungeon(state),
        GameKind::River => game_side_scroll(
            state,
            "River Raid",
            "/A\\",
            "#",
            Some("F"),
            "WASD move, grab fuel",
        ),
        GameKind::Memory => game_memory(state),
        GameKind::Number => game_number(state),
        GameKind::Circuit => game_circuit(state),
        GameKind::Orbit => game_orbit(state),
        GameKind::BlockDrop => game_block_drop(state),
        GameKind::CometCatcher => {
            falling_game(state, "Comet Catcher", "[@]", Some("*"), "!", 15, false)
        }
        GameKind::BombSweeper => grid_exit_game(state, "Bomb Sweeper", true, false),
        GameKind::NeonDrift => game_side_scroll(
            state,
            "Neon Drift",
            "/A\\",
            "|",
            Some("+"),
            "WASD drift, grab boosts",
        ),
        GameKind::CargoCatch => {
            falling_game(state, "Cargo Catch", "[_]", Some("[]"), "XX", 12, false)
        }
        GameKind::GemRush => falling_game(state, "Gem Rush", "(@)", Some("$"), "x", 15, false),
        GameKind::TrapRunner => grid_exit_game(state, "Trap Runner", true, false),
        GameKind::ReactorTrace => grid_exit_game(state, "Reactor Trace", false, true),
        GameKind::DroneDodge => {
            game_side_scroll(state, "Drone Dodge", "<D>", "*", None, "WASD evade")
        }
        GameKind::PearlDiver => {
            falling_game(state, "Pearl Diver", "{O}", Some("o"), "!", 15, false)
        }
        GameKind::SolarSailer => game_side_scroll(
            state,
            "Solar Sailer",
            "/S\\",
            "*",
            Some("+"),
            "WASD sail, grab charge",
        ),
        GameKind::VaultEscape => grid_exit_game(state, "Vault Escape", false, false),
        GameKind::DataStorm => falling_game(state, "Data Storm", "[#]", Some("D"), "E", 10, false),
        GameKind::PixelPop => game_pixel_pop(state),
        GameKind::BugHunt => game_bug_hunt(state),
        GameKind::FuelRun => game_side_scroll(
            state,
            "Fuel Run",
            "/A\\",
            "#",
            Some("F"),
            "WASD fly, grab fuel",
        ),
        GameKind::SparkChase => {
            game_side_scroll(state, "Spark Chase", "<+>", "*", Some("o"), "WASD weave")
        }
        GameKind::IceSlide => grid_exit_game(state, "Ice Slide", false, false),
        GameKind::SignalTrace => grid_exit_game(state, "Signal Trace", false, true),
        GameKind::OrbitalCourier => game_side_scroll(
            state,
            "Orbital Courier",
            "[O]",
            "*",
            Some("@"),
            "WASD courier",
        ),
        GameKind::RainRunner => {
            falling_game(state, "Rain Runner", "/A\\", Some("*"), "!", 12, false)
        }
        GameKind::ByteBlaster => game_byte_blaster(state),
        GameKind::StormSurge => game_side_scroll(
            state,
            "Storm Surge",
            "/A\\",
            "~",
            Some("F"),
            "WASD surf, grab fuel",
        ),
        GameKind::CrystalCavern => grid_exit_game(state, "Crystal Cavern", false, false),
        GameKind::TicTacToe => game_tic_tac_toe(state),
        GameKind::Chess => game_chess(state),
        GameKind::Checkers => game_checkers(state),
        GameKind::Micro(index) => {
            if let Some(spec) = MICRO_GAMES.get(index) {
                play_micro_game(state, game.name, *spec);
            } else {
                let _ = wait_menu(
                    state,
                    game.name,
                    &["This game pack entry is missing.".to_string()],
                    false,
                );
            }
        }
    }
}

fn play_micro_game(state: &mut AppState, name: &str, spec: MicroGame) {
    match spec.mode {
        MicroMode::ConnectFour => game_connect_four(state, name),
        MicroMode::WordGuess(kind) => game_word_guess(state, name, kind),
        MicroMode::Blackjack => game_blackjack(state, name),
        MicroMode::BlackjackBlitz => game_blackjack_blitz(state, name),
        MicroMode::Battleship => game_battleship(state, name),
        MicroMode::TowerStack => game_tower_stack(state, name),
        MicroMode::LightsOut => game_lights_out(state, name),
        MicroMode::SlidePuzzle => game_slide_puzzle(state, name),
        MicroMode::DominoChain => game_domino_chain(state, name),
        MicroMode::MiniGolf => game_mini_golf(state, name),
        MicroMode::Darts => game_darts(state, name),
        MicroMode::Mancala => game_mancala(state, name),
        MicroMode::MiniSudoku => game_mini_sudoku(state, name),
        MicroMode::Reversi => game_reversi(state, name),
        MicroMode::Bowling => game_bowling(state, name),
        MicroMode::SkeeBall => game_skee_ball(state, name),
        MicroMode::Keeper => game_keeper(state, name),
        MicroMode::Quest(kind) => game_micro_quest(state, name, kind),
        MicroMode::Lane(kind) => game_micro_lane(state, name, kind),
        MicroMode::Catch(kind) => game_micro_catch(state, name, kind),
        MicroMode::Aim(kind) => game_micro_aim(state, name, kind),
        MicroMode::Sequence(kind) => game_micro_sequence(state, name, kind),
    }
}

fn game_tic_tac_toe(state: &mut AppState) {
    if !require_size(state, 20, 54, "Tic Tac Toe") {
        return;
    }
    loop {
        let mut board = [' '; 9];
        let mut cursor = 4usize;
        let mut message = "Your move. You are X.".to_string();
        let mut moves = 0u32;
        let mut finished = false;
        let mut score = 0u32;
        while !finished {
            draw_tic_tac_toe(state, &board, cursor, &message, score);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 3 => cursor -= 3,
                    Key::Down | Key::Char('s') if cursor < 6 => cursor += 3,
                    Key::Left | Key::Char('a') if cursor % 3 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 3 < 2 => cursor += 1,
                    Key::Enter | Key::Space => {
                        if board[cursor] != ' ' {
                            message = "That square is taken.".to_string();
                            play_sound(state, "wall");
                            continue;
                        }
                        board[cursor] = 'X';
                        moves += 1;
                        if ttt_winner(&board) == Some('X') {
                            score = 400u32.saturating_sub(moves * 20);
                            message = "You made three in a row.".to_string();
                            play_sound(state, "score");
                            finished = true;
                            continue;
                        }
                        if ttt_full(&board) {
                            score = 125;
                            message = "Draw board. No clean line.".to_string();
                            finished = true;
                            continue;
                        }
                        let cpu = ttt_cpu_move(&board, state);
                        board[cpu] = 'O';
                        if ttt_winner(&board) == Some('O') {
                            score = 25;
                            message = "CPU found the line.".to_string();
                            play_sound(state, "alert");
                            finished = true;
                        } else if ttt_full(&board) {
                            score = 125;
                            message = "Draw board. No clean line.".to_string();
                            finished = true;
                        } else {
                            message = "CPU moved. Find the line.".to_string();
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        draw_tic_tac_toe(state, &board, cursor, &message, score);
        record_score(state, "Tic Tac Toe", score);
        if !wait_menu(
            state,
            "Tic Tac Toe",
            &[message, format!("Score: {score}")],
            true,
        ) {
            return;
        }
    }
}

fn ttt_winner(board: &[char; 9]) -> Option<char> {
    const LINES: [[usize; 3]; 8] = [
        [0, 1, 2],
        [3, 4, 5],
        [6, 7, 8],
        [0, 3, 6],
        [1, 4, 7],
        [2, 5, 8],
        [0, 4, 8],
        [2, 4, 6],
    ];
    for line in LINES {
        let mark = board[line[0]];
        if mark != ' ' && board[line[1]] == mark && board[line[2]] == mark {
            return Some(mark);
        }
    }
    None
}

fn ttt_full(board: &[char; 9]) -> bool {
    board.iter().all(|&mark| mark != ' ')
}

fn ttt_cpu_move(board: &[char; 9], state: &mut AppState) -> usize {
    for mark in ['O', 'X'] {
        for index in 0..9 {
            if board[index] != ' ' {
                continue;
            }
            let mut test = *board;
            test[index] = mark;
            if ttt_winner(&test) == Some(mark) {
                return index;
            }
        }
    }
    if board[4] == ' ' {
        return 4;
    }
    let corners: Vec<usize> = [0, 2, 6, 8]
        .into_iter()
        .filter(|&index| board[index] == ' ')
        .collect();
    if !corners.is_empty() {
        return corners[state.rng.usize(corners.len())];
    }
    let empties: Vec<usize> = (0..9).filter(|&index| board[index] == ' ').collect();
    empties[state.rng.usize(empties.len())]
}

fn draw_tic_tac_toe(state: &AppState, board: &[char; 9], cursor: usize, message: &str, score: u32) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 1, "TIC TAC TOE", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        3,
        &format!("Score {score}   Move cursor, Enter/Space place X, Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let top = rows / 2 - 4;
    let left = cols / 2 - 10;
    for y in 0..3 {
        for x in 0..3 {
            let index = y * 3 + x;
            let mark = if board[index] == ' ' {
                (index + 1).to_string()
            } else {
                board[index].to_string()
            };
            let text = format!("  {mark}  ");
            let row = top + y * 3;
            let col = left + x * 7;
            if index == cursor {
                put_inv(&mut buf, row, col, &text, &theme, Role::Highlight);
            } else {
                let role = match board[index] {
                    'X' => Role::Success,
                    'O' => Role::Danger,
                    _ => Role::Muted,
                };
                put(&mut buf, row, col, &text, &theme, role, board[index] != ' ');
            }
        }
    }
    for y in [top + 1, top + 4] {
        put(
            &mut buf,
            y,
            left,
            "-----+-----+-----",
            &theme,
            Role::Muted,
            false,
        );
    }
    for y in 0..7 {
        put(&mut buf, top + y, left + 5, "|", &theme, Role::Muted, false);
        put(
            &mut buf,
            top + y,
            left + 12,
            "|",
            &theme,
            Role::Muted,
            false,
        );
    }
    center(
        &mut buf,
        top + 9,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_chess(state: &mut AppState) {
    if !require_size(state, 24, 78, "Chess") {
        return;
    }
    loop {
        let mut board = chess_initial_board();
        let mut cursor = chess_idx(4, 6);
        let mut selected = None;
        let mut player_captures = 0u32;
        let mut cpu_captures = 0u32;
        let mut ply = 0u32;
        let mut message = "White to move. Select a piece, then a destination.".to_string();
        let mut result = None;
        while result.is_none() {
            draw_chess(
                state,
                &board,
                cursor,
                selected,
                &message,
                player_captures,
                cpu_captures,
            );
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 8 => cursor -= 8,
                    Key::Down | Key::Char('s') if cursor < 56 => cursor += 8,
                    Key::Left | Key::Char('a') if cursor % 8 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 8 < 7 => cursor += 1,
                    Key::Enter | Key::Space => {
                        if let Some(from) = selected {
                            if from == cursor {
                                selected = None;
                                message = "Selection cleared.".to_string();
                                continue;
                            }
                            if chess_legal_move(&board, from, cursor, true) {
                                let captured = board[cursor];
                                board = chess_make_move(board, from, cursor);
                                if captured != '.' {
                                    player_captures += chess_piece_value(captured) as u32;
                                }
                                ply += 1;
                                selected = None;
                                play_sound(state, "score");
                                let black_moves = chess_all_legal_moves(&board, false);
                                if black_moves.is_empty() {
                                    result = Some(if chess_in_check(&board, false) {
                                        "Checkmate. You won.".to_string()
                                    } else {
                                        "Stalemate. No legal CPU move.".to_string()
                                    });
                                    continue;
                                }
                                if let Some((cpu_from, cpu_to)) = chess_pick_cpu_move(state, &board)
                                {
                                    let cpu_capture = board[cpu_to];
                                    board = chess_make_move(board, cpu_from, cpu_to);
                                    if cpu_capture != '.' {
                                        cpu_captures += chess_piece_value(cpu_capture) as u32;
                                        play_sound(state, "alert");
                                    } else {
                                        play_sound(state, "paddle");
                                    }
                                    ply += 1;
                                    let white_moves = chess_all_legal_moves(&board, true);
                                    if white_moves.is_empty() {
                                        result = Some(if chess_in_check(&board, true) {
                                            "Checkmate. CPU won.".to_string()
                                        } else {
                                            "Stalemate. No legal white move.".to_string()
                                        });
                                    } else if chess_in_check(&board, true) {
                                        message = "CPU moved. You are in check.".to_string();
                                    } else {
                                        message = "CPU moved. White to move.".to_string();
                                    }
                                }
                            } else {
                                message = "Illegal chess move.".to_string();
                                play_sound(state, "wall");
                            }
                        } else if chess_is_white_piece(board[cursor]) {
                            selected = Some(cursor);
                            message = "Piece selected. Choose a legal destination.".to_string();
                        } else {
                            message = "Select one of your white pieces.".to_string();
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let result = result.unwrap_or_else(|| "Board ended.".to_string());
        draw_chess(
            state,
            &board,
            cursor,
            selected,
            &result,
            player_captures,
            cpu_captures,
        );
        let score = 300u32
            .saturating_add(player_captures * 35)
            .saturating_sub(cpu_captures * 25)
            .saturating_sub(ply * 2);
        record_score(state, "Chess", score);
        if !wait_menu(
            state,
            "Chess",
            &[
                result,
                format!("Material score: {player_captures}-{cpu_captures}"),
                format!("Score: {score}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn chess_initial_board() -> [char; 64] {
    let mut board = ['.'; 64];
    let back = ['r', 'n', 'b', 'q', 'k', 'b', 'n', 'r'];
    for x in 0..8 {
        board[chess_idx(x, 0)] = back[x];
        board[chess_idx(x, 1)] = 'p';
        board[chess_idx(x, 6)] = 'P';
        board[chess_idx(x, 7)] = back[x].to_ascii_uppercase();
    }
    board
}

fn chess_idx(x: usize, y: usize) -> usize {
    y * 8 + x
}

fn chess_xy(index: usize) -> (i32, i32) {
    ((index % 8) as i32, (index / 8) as i32)
}

fn chess_is_white_piece(piece: char) -> bool {
    piece.is_ascii_uppercase()
}

fn chess_is_black_piece(piece: char) -> bool {
    piece.is_ascii_lowercase()
}

fn chess_same_color(a: char, b: char) -> bool {
    (chess_is_white_piece(a) && chess_is_white_piece(b))
        || (chess_is_black_piece(a) && chess_is_black_piece(b))
}

fn chess_piece_value(piece: char) -> i32 {
    match piece.to_ascii_lowercase() {
        'p' => 1,
        'n' | 'b' => 3,
        'r' => 5,
        'q' => 9,
        'k' => 20,
        _ => 0,
    }
}

fn chess_make_move(mut board: [char; 64], from: usize, to: usize) -> [char; 64] {
    let mut piece = board[from];
    board[from] = '.';
    let (_, to_y) = chess_xy(to);
    if piece == 'P' && to_y == 0 {
        piece = 'Q';
    } else if piece == 'p' && to_y == 7 {
        piece = 'q';
    }
    board[to] = piece;
    board
}

fn chess_legal_move(board: &[char; 64], from: usize, to: usize, white_turn: bool) -> bool {
    if from == to || from >= 64 || to >= 64 {
        return false;
    }
    let piece = board[from];
    if piece == '.' {
        return false;
    }
    if white_turn != chess_is_white_piece(piece) {
        return false;
    }
    let target = board[to];
    if target != '.' && chess_same_color(piece, target) {
        return false;
    }
    if target.to_ascii_lowercase() == 'k' {
        return false;
    }
    if !chess_piece_shape_legal(board, from, to, piece, target) {
        return false;
    }
    let next = chess_make_move(*board, from, to);
    !chess_in_check(&next, white_turn)
}

fn chess_piece_shape_legal(
    board: &[char; 64],
    from: usize,
    to: usize,
    piece: char,
    target: char,
) -> bool {
    let (fx, fy) = chess_xy(from);
    let (tx, ty) = chess_xy(to);
    let dx = tx - fx;
    let dy = ty - fy;
    match piece.to_ascii_lowercase() {
        'p' => {
            let dir = if chess_is_white_piece(piece) { -1 } else { 1 };
            let start_row = if chess_is_white_piece(piece) { 6 } else { 1 };
            if dx == 0 && dy == dir && target == '.' {
                return true;
            }
            if dx == 0 && dy == dir * 2 && fy == start_row && target == '.' {
                let mid = chess_idx(fx as usize, (fy + dir) as usize);
                return board[mid] == '.';
            }
            dy == dir && dx.abs() == 1 && target != '.'
        }
        'n' => (dx.abs() == 1 && dy.abs() == 2) || (dx.abs() == 2 && dy.abs() == 1),
        'b' => dx.abs() == dy.abs() && chess_path_clear(board, fx, fy, tx, ty),
        'r' => (dx == 0 || dy == 0) && chess_path_clear(board, fx, fy, tx, ty),
        'q' => {
            (dx == 0 || dy == 0 || dx.abs() == dy.abs()) && chess_path_clear(board, fx, fy, tx, ty)
        }
        'k' => dx.abs() <= 1 && dy.abs() <= 1,
        _ => false,
    }
}

fn chess_path_clear(board: &[char; 64], fx: i32, fy: i32, tx: i32, ty: i32) -> bool {
    let step_x = (tx - fx).signum();
    let step_y = (ty - fy).signum();
    let mut x = fx + step_x;
    let mut y = fy + step_y;
    while (x, y) != (tx, ty) {
        if board[chess_idx(x as usize, y as usize)] != '.' {
            return false;
        }
        x += step_x;
        y += step_y;
    }
    true
}

fn chess_in_check(board: &[char; 64], white_king: bool) -> bool {
    let king = if white_king { 'K' } else { 'k' };
    let Some(king_index) = board.iter().position(|&piece| piece == king) else {
        return true;
    };
    chess_square_attacked(board, king_index, !white_king)
}

fn chess_square_attacked(board: &[char; 64], square: usize, by_white: bool) -> bool {
    let (tx, ty) = chess_xy(square);
    for from in 0..64 {
        let piece = board[from];
        if piece == '.' || chess_is_white_piece(piece) != by_white {
            continue;
        }
        let (fx, fy) = chess_xy(from);
        let dx = tx - fx;
        let dy = ty - fy;
        let attacks = match piece.to_ascii_lowercase() {
            'p' => {
                let dir = if by_white { -1 } else { 1 };
                dy == dir && dx.abs() == 1
            }
            'n' => (dx.abs() == 1 && dy.abs() == 2) || (dx.abs() == 2 && dy.abs() == 1),
            'b' => dx.abs() == dy.abs() && chess_path_clear(board, fx, fy, tx, ty),
            'r' => (dx == 0 || dy == 0) && chess_path_clear(board, fx, fy, tx, ty),
            'q' => {
                (dx == 0 || dy == 0 || dx.abs() == dy.abs())
                    && chess_path_clear(board, fx, fy, tx, ty)
            }
            'k' => dx.abs() <= 1 && dy.abs() <= 1,
            _ => false,
        };
        if attacks {
            return true;
        }
    }
    false
}

fn chess_all_legal_moves(board: &[char; 64], white_turn: bool) -> Vec<(usize, usize)> {
    let mut moves = Vec::new();
    for from in 0..64 {
        let piece = board[from];
        if piece == '.' || chess_is_white_piece(piece) != white_turn {
            continue;
        }
        for to in 0..64 {
            if chess_legal_move(board, from, to, white_turn) {
                moves.push((from, to));
            }
        }
    }
    moves
}

fn chess_pick_cpu_move(state: &mut AppState, board: &[char; 64]) -> Option<(usize, usize)> {
    let moves = chess_all_legal_moves(board, false);
    if moves.is_empty() {
        return None;
    }
    let mut best_score = i32::MIN;
    let mut best_moves = Vec::new();
    for (from, to) in moves {
        let mut score = chess_piece_value(board[to]) * 10;
        let next = chess_make_move(*board, from, to);
        if chess_in_check(&next, true) {
            score += 8;
        }
        let (x, y) = chess_xy(to);
        score += 4 - (x - 3).abs().min(4);
        score += 4 - (y - 3).abs().min(4);
        score += state.rng.range(0, 2);
        if score > best_score {
            best_score = score;
            best_moves.clear();
            best_moves.push((from, to));
        } else if score == best_score {
            best_moves.push((from, to));
        }
    }
    Some(best_moves[state.rng.usize(best_moves.len())])
}

fn draw_chess(
    state: &AppState,
    board: &[char; 64],
    cursor: usize,
    selected: Option<usize>,
    message: &str,
    player_captures: u32,
    cpu_captures: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 6;
    let left = cols / 2 - 17;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "CHESS", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!(
            "White pieces uppercase   Captures {player_captures}-{cpu_captures}   Enter select/move"
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 4,
        12,
        39,
        "BOARD",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for y in 0..8 {
        put(
            &mut buf,
            top + y,
            left - 2,
            &(8 - y).to_string(),
            &theme,
            Role::Muted,
            false,
        );
        for x in 0..8 {
            let index = chess_idx(x, y);
            let piece = board[index];
            let shown = if piece == '.' { "." } else { "" };
            let text = if piece == '.' {
                format!(" {shown} ")
            } else {
                format!(" {piece} ")
            };
            let role = if Some(index) == selected {
                Role::Highlight
            } else if chess_is_white_piece(piece) {
                Role::Success
            } else if chess_is_black_piece(piece) {
                Role::Danger
            } else if (x + y) % 2 == 0 {
                Role::Muted
            } else {
                Role::Normal
            };
            if index == cursor {
                put_inv(
                    &mut buf,
                    top + y,
                    left + x * 4,
                    &text,
                    &theme,
                    Role::Highlight,
                );
            } else {
                put(
                    &mut buf,
                    top + y,
                    left + x * 4,
                    &text,
                    &theme,
                    role,
                    piece != '.' || Some(index) == selected,
                );
            }
        }
    }
    put(
        &mut buf,
        top + 9,
        left,
        "  a   b   c   d   e   f   g   h",
        &theme,
        Role::Muted,
        false,
    );
    center(
        &mut buf,
        top + 11,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    center(
        &mut buf,
        top + 13,
        "No castling or en passant; pawns promote to queens.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_checkers(state: &mut AppState) {
    if !require_size(state, 24, 78, "Checkers") {
        return;
    }
    loop {
        let mut board = checkers_initial_board();
        let mut cursor = checkers_idx(1, 5);
        let mut selected = None;
        let mut player_captures = 0u32;
        let mut cpu_captures = 0u32;
        let mut message = "White to move. Captures are mandatory.".to_string();
        let mut result = None;
        while result.is_none() {
            draw_checkers(
                state,
                &board,
                cursor,
                selected,
                &message,
                player_captures,
                cpu_captures,
            );
            if checkers_legal_moves(&board, true).is_empty() {
                result = Some("No legal white move. CPU wins.".to_string());
                continue;
            }
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 8 => cursor -= 8,
                    Key::Down | Key::Char('s') if cursor < 56 => cursor += 8,
                    Key::Left | Key::Char('a') if cursor % 8 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 8 < 7 => cursor += 1,
                    Key::Enter | Key::Space => {
                        if let Some(from) = selected {
                            if from == cursor {
                                selected = None;
                                message = "Selection cleared.".to_string();
                                continue;
                            }
                            let legal = checkers_legal_moves(&board, true);
                            if let Some((_, _, captured)) =
                                legal.iter().copied().find(|&(move_from, move_to, _)| {
                                    move_from == from && move_to == cursor
                                })
                            {
                                checkers_apply_move(&mut board, from, cursor, captured);
                                if captured.is_some() {
                                    player_captures += 1;
                                    play_sound(state, "score");
                                } else {
                                    play_sound(state, "paddle");
                                }
                                selected = None;
                                if checkers_count(&board, false) == 0 {
                                    result = Some("You cleared every black piece.".to_string());
                                    continue;
                                }
                                let cpu_moves = checkers_legal_moves(&board, false);
                                if cpu_moves.is_empty() {
                                    result = Some("CPU has no legal move. You win.".to_string());
                                    continue;
                                }
                                let (cpu_from, cpu_to, cpu_capture) =
                                    checkers_pick_cpu_move(state, &board, &cpu_moves);
                                checkers_apply_move(&mut board, cpu_from, cpu_to, cpu_capture);
                                if cpu_capture.is_some() {
                                    cpu_captures += 1;
                                    play_sound(state, "alert");
                                }
                                if checkers_count(&board, true) == 0 {
                                    result = Some("CPU captured your last piece.".to_string());
                                } else {
                                    message = "CPU moved. White to move.".to_string();
                                }
                            } else {
                                message = "Illegal checkers move.".to_string();
                                play_sound(state, "wall");
                            }
                        } else if checkers_is_white_piece(board[cursor]) {
                            selected = Some(cursor);
                            message = "Piece selected. Move diagonally or jump.".to_string();
                        } else {
                            message = "Select one of your white checkers.".to_string();
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let result = result.unwrap_or_else(|| "Checkers ended.".to_string());
        draw_checkers(
            state,
            &board,
            cursor,
            selected,
            &result,
            player_captures,
            cpu_captures,
        );
        let score = 250u32 + player_captures * 80 - cpu_captures.min(3) * 35;
        record_score(state, "Checkers", score);
        if !wait_menu(
            state,
            "Checkers",
            &[
                result,
                format!("Captures: {player_captures}-{cpu_captures}"),
                format!("Score: {score}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn checkers_initial_board() -> [char; 64] {
    let mut board = ['.'; 64];
    for y in 0..3 {
        for x in 0..8 {
            if (x + y) % 2 == 1 {
                board[checkers_idx(x, y)] = 'b';
            }
        }
    }
    for y in 5..8 {
        for x in 0..8 {
            if (x + y) % 2 == 1 {
                board[checkers_idx(x, y)] = 'w';
            }
        }
    }
    board
}

fn checkers_idx(x: usize, y: usize) -> usize {
    y * 8 + x
}

fn checkers_xy(index: usize) -> (i32, i32) {
    ((index % 8) as i32, (index / 8) as i32)
}

fn checkers_is_white_piece(piece: char) -> bool {
    matches!(piece, 'w' | 'W')
}

fn checkers_is_black_piece(piece: char) -> bool {
    matches!(piece, 'b' | 'B')
}

fn checkers_is_king(piece: char) -> bool {
    matches!(piece, 'W' | 'B')
}

fn checkers_dirs(piece: char) -> &'static [(i32, i32)] {
    match piece {
        'w' => &[(-1, -1), (1, -1)],
        'b' => &[(-1, 1), (1, 1)],
        'W' | 'B' => &[(-1, -1), (1, -1), (-1, 1), (1, 1)],
        _ => &[],
    }
}

fn checkers_legal_moves(
    board: &[char; 64],
    white_turn: bool,
) -> Vec<(usize, usize, Option<usize>)> {
    let mut moves = Vec::new();
    let mut captures = Vec::new();
    for from in 0..64 {
        let piece = board[from];
        if piece == '.' || checkers_is_white_piece(piece) != white_turn {
            continue;
        }
        let (fx, fy) = checkers_xy(from);
        for &(dx, dy) in checkers_dirs(piece) {
            let nx = fx + dx;
            let ny = fy + dy;
            if !(0..8).contains(&nx) || !(0..8).contains(&ny) {
                continue;
            }
            let step = checkers_idx(nx as usize, ny as usize);
            if board[step] == '.' {
                moves.push((from, step, None));
            } else if checkers_is_white_piece(board[step]) != white_turn {
                let jx = fx + dx * 2;
                let jy = fy + dy * 2;
                if (0..8).contains(&jx) && (0..8).contains(&jy) {
                    let jump = checkers_idx(jx as usize, jy as usize);
                    if board[jump] == '.' {
                        captures.push((from, jump, Some(step)));
                    }
                }
            }
        }
    }
    if captures.is_empty() {
        moves
    } else {
        captures
    }
}

fn checkers_apply_move(board: &mut [char; 64], from: usize, to: usize, captured: Option<usize>) {
    let mut piece = board[from];
    board[from] = '.';
    if let Some(captured) = captured {
        board[captured] = '.';
    }
    let (_, y) = checkers_xy(to);
    if piece == 'w' && y == 0 {
        piece = 'W';
    } else if piece == 'b' && y == 7 {
        piece = 'B';
    }
    board[to] = piece;
}

fn checkers_pick_cpu_move(
    state: &mut AppState,
    board: &[char; 64],
    moves: &[(usize, usize, Option<usize>)],
) -> (usize, usize, Option<usize>) {
    let mut best_score = i32::MIN;
    let mut best_moves = Vec::new();
    for &(from, to, captured) in moves {
        let piece = board[from];
        let (_, y) = checkers_xy(to);
        let mut score = if captured.is_some() { 10 } else { 0 };
        if piece == 'b' && y == 7 {
            score += 6;
        }
        if checkers_is_king(piece) {
            score += 2;
        }
        score += state.rng.range(0, 2);
        if score > best_score {
            best_score = score;
            best_moves.clear();
            best_moves.push((from, to, captured));
        } else if score == best_score {
            best_moves.push((from, to, captured));
        }
    }
    best_moves[state.rng.usize(best_moves.len())]
}

fn checkers_count(board: &[char; 64], white: bool) -> usize {
    board
        .iter()
        .filter(|&&piece| {
            if white {
                checkers_is_white_piece(piece)
            } else {
                checkers_is_black_piece(piece)
            }
        })
        .count()
}

fn draw_checkers(
    state: &AppState,
    board: &[char; 64],
    cursor: usize,
    selected: Option<usize>,
    message: &str,
    player_captures: u32,
    cpu_captures: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 6;
    let left = cols / 2 - 17;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "CHECKERS", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("White w/W   Black b/B   Captures {player_captures}-{cpu_captures}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 4,
        12,
        39,
        "BOARD",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for y in 0..8 {
        put(
            &mut buf,
            top + y,
            left - 2,
            &(8 - y).to_string(),
            &theme,
            Role::Muted,
            false,
        );
        for x in 0..8 {
            let index = checkers_idx(x, y);
            let piece = board[index];
            let marker = if (x + y) % 2 == 0 { " " } else { "." };
            let text = if piece == '.' {
                format!(" {marker} ")
            } else {
                format!(" {piece} ")
            };
            let role = if Some(index) == selected {
                Role::Highlight
            } else if checkers_is_white_piece(piece) {
                Role::Success
            } else if checkers_is_black_piece(piece) {
                Role::Danger
            } else {
                Role::Muted
            };
            if index == cursor {
                put_inv(
                    &mut buf,
                    top + y,
                    left + x * 4,
                    &text,
                    &theme,
                    Role::Highlight,
                );
            } else {
                put(
                    &mut buf,
                    top + y,
                    left + x * 4,
                    &text,
                    &theme,
                    role,
                    piece != '.' || Some(index) == selected,
                );
            }
        }
    }
    put(
        &mut buf,
        top + 9,
        left,
        "  a   b   c   d   e   f   g   h",
        &theme,
        Role::Muted,
        false,
    );
    center(
        &mut buf,
        top + 11,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    center(
        &mut buf,
        top + 13,
        "Enter selects/moves. Men crown into kings on the far edge.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_connect_four(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 60, name) {
        return;
    }
    loop {
        let mut board = [' '; 42];
        let mut cursor = 3usize;
        let mut moves = 0u32;
        let mut score = 0u32;
        let mut message = "Drop X pieces. Connect four before the CPU.".to_string();
        let mut finished = false;
        while !finished {
            draw_connect_four(state, name, &board, cursor, &message, score);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Left | Key::Char('a') if cursor > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor < 6 => cursor += 1,
                    Key::Enter | Key::Space => {
                        if !cf_drop(&mut board, cursor, 'X') {
                            message = "That column is full.".to_string();
                            play_sound(state, "wall");
                            continue;
                        }
                        moves += 1;
                        if cf_winner(&board) == Some('X') {
                            score = 700u32.saturating_sub(moves * 18);
                            message = "You connected four.".to_string();
                            play_sound(state, "score");
                            finished = true;
                            continue;
                        }
                        if cf_full(&board) {
                            score = 150;
                            message = "Board filled. Draw.".to_string();
                            finished = true;
                            continue;
                        }
                        let cpu_col = cf_cpu_column(&board, state);
                        let _ = cf_drop(&mut board, cpu_col, 'O');
                        if cf_winner(&board) == Some('O') {
                            score = 40;
                            message = "CPU connected four.".to_string();
                            play_sound(state, "alert");
                            finished = true;
                        } else if cf_full(&board) {
                            score = 150;
                            message = "Board filled. Draw.".to_string();
                            finished = true;
                        } else {
                            message = format!("CPU dropped in column {}.", cpu_col + 1);
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        draw_connect_four(state, name, &board, cursor, &message, score);
        record_score(state, name, score);
        if !wait_menu(state, name, &[message, format!("Score: {score}")], true) {
            return;
        }
    }
}

fn cf_drop(board: &mut [char; 42], col: usize, mark: char) -> bool {
    for row in (0..6).rev() {
        let index = row * 7 + col;
        if board[index] == ' ' {
            board[index] = mark;
            return true;
        }
    }
    false
}

fn cf_full(board: &[char; 42]) -> bool {
    board.iter().all(|&cell| cell != ' ')
}

fn cf_winner(board: &[char; 42]) -> Option<char> {
    let directions = [(1i32, 0i32), (0, 1), (1, 1), (1, -1)];
    for row in 0..6 {
        for col in 0..7 {
            let mark = board[row * 7 + col];
            if mark == ' ' {
                continue;
            }
            for (dx, dy) in directions {
                let mut count = 1;
                for step in 1..4 {
                    let x = col as i32 + dx * step;
                    let y = row as i32 + dy * step;
                    if !(0..7).contains(&x) || !(0..6).contains(&y) {
                        break;
                    }
                    if board[y as usize * 7 + x as usize] == mark {
                        count += 1;
                    }
                }
                if count == 4 {
                    return Some(mark);
                }
            }
        }
    }
    None
}

fn cf_cpu_column(board: &[char; 42], state: &mut AppState) -> usize {
    let available: Vec<usize> = (0..7).filter(|&col| board[col] == ' ').collect();
    for mark in ['O', 'X'] {
        for &col in &available {
            let mut test = *board;
            let _ = cf_drop(&mut test, col, mark);
            if cf_winner(&test) == Some(mark) {
                return col;
            }
        }
    }
    if available.contains(&3) {
        3
    } else {
        available[state.rng.usize(available.len())]
    }
}

fn draw_connect_four(
    state: &AppState,
    name: &str,
    board: &[char; 42],
    cursor: usize,
    message: &str,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Score {score}   A/D choose column   Space drops   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let top = rows / 2 - 7;
    let left = cols / 2 - 15;
    for col in 0..7 {
        let marker = if col == cursor { "v" } else { " " };
        put(
            &mut buf,
            top,
            left + col * 4 + 1,
            marker,
            &theme,
            Role::Highlight,
            true,
        );
    }
    for row in 0..6 {
        for col in 0..7 {
            let mark = board[row * 7 + col];
            let text = format!("[{}]", if mark == ' ' { '.' } else { mark });
            let role = match mark {
                'X' => Role::Success,
                'O' => Role::Danger,
                _ => Role::Muted,
            };
            put(
                &mut buf,
                top + 2 + row * 2,
                left + col * 4,
                &text,
                &theme,
                role,
                mark != ' ',
            );
        }
    }
    center(
        &mut buf,
        top + 15,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_word_guess(state: &mut AppState, name: &str, kind: WordKind) {
    const WORDS: [&str; 18] = [
        "arcade", "terminal", "cipher", "wizard", "volcano", "galaxy", "crystal", "signal",
        "pirate", "robot", "jungle", "dragon", "mirror", "rocket", "castle", "meteor", "circuit",
        "potion",
    ];
    const VAULT_WORDS: [&str; 10] = [
        "cipher", "signal", "circuit", "terminal", "rocket", "galaxy", "mirror", "crystal",
        "arcade", "meteor",
    ];
    loop {
        let word = match kind {
            WordKind::Vault => VAULT_WORDS[state.rng.usize(VAULT_WORDS.len())],
            WordKind::Hangman => WORDS[state.rng.usize(WORDS.len())],
        };
        let mut guessed = HashSet::new();
        let mut wrong = Vec::new();
        let max_wrong = match kind {
            WordKind::Vault => match state.difficulty_index {
                0 => 5,
                1 => 4,
                _ => 3,
            },
            WordKind::Hangman => match state.difficulty_index {
                0 => 8,
                1 => 6,
                _ => 5,
            },
        };
        let mut message = match kind {
            WordKind::Vault => "Crack the vault with fewer misses.".to_string(),
            WordKind::Hangman => "Type letters. Esc quits.".to_string(),
        };
        let mut won = false;
        while wrong.len() < max_wrong && !won {
            draw_word_guess(state, name, word, &guessed, &wrong, max_wrong, &message);
            let Some(key) = wait_for_text_key() else {
                return;
            };
            match key {
                Key::Esc => return,
                Key::Char(ch) if ch.is_ascii_alphabetic() => {
                    let ch = ch.to_ascii_lowercase();
                    if guessed.contains(&ch) || wrong.contains(&ch) {
                        message = format!("Already tried {}.", ch.to_ascii_uppercase());
                    } else if word.contains(ch) {
                        guessed.insert(ch);
                        message = "Good letter.".to_string();
                        play_sound(state, "score");
                    } else {
                        wrong.push(ch);
                        message = "Nope.".to_string();
                        play_sound(state, "wall");
                    }
                    won = word.chars().all(|letter| guessed.contains(&letter));
                }
                _ => {}
            }
        }
        let score = if won {
            let base: u32 = if matches!(kind, WordKind::Vault) {
                650
            } else {
                500
            };
            base.saturating_sub(wrong.len() as u32 * 35) + guessed.len() as u32 * 10
        } else {
            guessed.len() as u32 * 15
        };
        record_score(state, name, score);
        let result = if won {
            format!("Solved: {word}. Score: {score}")
        } else {
            format!("Word was {word}. Score: {score}")
        };
        if !wait_menu(state, name, &[result], true) {
            return;
        }
    }
}

fn wait_for_text_key() -> Option<Key> {
    loop {
        if let Some(key) = read_text_key() {
            return Some(key);
        }
        thread::sleep(Duration::from_millis(15));
    }
}

fn draw_word_guess(
    state: &AppState,
    name: &str,
    word: &str,
    guessed: &HashSet<char>,
    wrong: &[char],
    max_wrong: usize,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        rows / 2 - 7,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    let hidden: String = word
        .chars()
        .map(|ch| {
            if guessed.contains(&ch) {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .flat_map(|ch| [ch, ' '])
        .collect();
    center(
        &mut buf,
        rows / 2 - 3,
        &hidden,
        &theme,
        Role::Highlight,
        true,
        cols,
    );
    let wrong_text: String = wrong
        .iter()
        .map(|ch| ch.to_ascii_uppercase().to_string())
        .collect::<Vec<_>>()
        .join(" ");
    center(
        &mut buf,
        rows / 2,
        &format!("Wrong {}/{}: {}", wrong.len(), max_wrong, wrong_text),
        &theme,
        if wrong.len() + 1 >= max_wrong {
            Role::Danger
        } else {
            Role::Accent
        },
        false,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 3,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 6,
        "Type letters. Esc returns to menu.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_blackjack(state: &mut AppState, name: &str) {
    loop {
        let mut player = vec![draw_blackjack_card(state), draw_blackjack_card(state)];
        let mut dealer = vec![draw_blackjack_card(state), draw_blackjack_card(state)];
        let mut message = "Space hits. Right/Down stands.".to_string();
        let mut standing = false;
        let mut finished = false;
        let mut score = 0u32;
        while !finished {
            draw_blackjack(state, name, &player, &dealer, standing, &message, score);
            if blackjack_value(&player) > 21 {
                message = "Bust. Dealer wins.".to_string();
                score = 10;
                finished = true;
                continue;
            }
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Enter | Key::Space | Key::Char('h') if !standing => {
                        player.push(draw_blackjack_card(state));
                        play_sound(state, "wall");
                    }
                    Key::Right | Key::Down | Key::Char('d') | Key::Char('s') => {
                        standing = true;
                        while blackjack_value(&dealer) < 17 {
                            dealer.push(draw_blackjack_card(state));
                        }
                        let player_value = blackjack_value(&player);
                        let dealer_value = blackjack_value(&dealer);
                        if dealer_value > 21 || player_value > dealer_value {
                            score = 250 + player_value as u32 * 5;
                            message = "You beat the dealer.".to_string();
                            play_sound(state, "score");
                        } else if player_value == dealer_value {
                            score = 100;
                            message = "Push.".to_string();
                        } else {
                            score = 25;
                            message = "Dealer holds the better hand.".to_string();
                            play_sound(state, "alert");
                        }
                        finished = true;
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        draw_blackjack(state, name, &player, &dealer, true, &message, score);
        record_score(state, name, score);
        if !wait_menu(state, name, &[message, format!("Score: {score}")], true) {
            return;
        }
    }
}

fn game_blackjack_blitz(state: &mut AppState, name: &str) {
    loop {
        let mut hand = Vec::new();
        let mut draws = 0u32;
        let mut message = "Space draws. Right/Down banks the hand.".to_string();
        let mut finished = false;
        while !finished {
            let total = blackjack_value(&hand);
            draw_blackjack_blitz(state, name, &hand, draws, &message);
            if total > 21 || total == 21 || draws >= 5 {
                finished = true;
                continue;
            }
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Enter | Key::Space => {
                        hand.push(draw_blackjack_card(state));
                        draws += 1;
                        play_sound(state, "wall");
                    }
                    Key::Right | Key::Down | Key::Char('d') | Key::Char('s') => {
                        finished = true;
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            let total = blackjack_value(&hand);
            message = if total > 21 {
                "Bust.".to_string()
            } else if total == 21 {
                "Blackjack.".to_string()
            } else {
                format!("Total {total}. Draw or bank.")
            };
        }
        let total = blackjack_value(&hand);
        let score = if total > 21 {
            0
        } else {
            600u32.saturating_sub((21 - total as i32).unsigned_abs() * 45) + draws * 15
        };
        draw_blackjack_blitz(state, name, &hand, draws, "Round over.");
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[format!("Total {total}. Score: {score}")],
            true,
        ) {
            return;
        }
    }
}

fn draw_blackjack_blitz(state: &AppState, name: &str, hand: &[u8], draws: u32, message: &str) {
    let (_, cols) = terminal_size();
    let theme = state.theme().clone();
    let cards = blackjack_hand_text(hand, false);
    let total = blackjack_value(hand);
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        5,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        9,
        &format!("Draws {draws}/5   Total {total}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    center(&mut buf, 12, &cards, &theme, Role::Success, true, cols);
    center(&mut buf, 16, message, &theme, Role::Secondary, true, cols);
    center(
        &mut buf,
        18,
        "Space draw   Right/Down bank   Q menu",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn draw_blackjack_card(state: &mut AppState) -> u8 {
    state.rng.range(1, 13) as u8
}

fn blackjack_value(cards: &[u8]) -> u8 {
    let mut total = 0u8;
    let mut aces = 0u8;
    for &card in cards {
        match card {
            1 => {
                total += 11;
                aces += 1;
            }
            11..=13 => total += 10,
            value => total += value,
        }
    }
    while total > 21 && aces > 0 {
        total -= 10;
        aces -= 1;
    }
    total
}

fn blackjack_card_label(card: u8) -> &'static str {
    match card {
        1 => "A",
        11 => "J",
        12 => "Q",
        13 => "K",
        10 => "10",
        9 => "9",
        8 => "8",
        7 => "7",
        6 => "6",
        5 => "5",
        4 => "4",
        3 => "3",
        _ => "2",
    }
}

fn blackjack_hand_text(cards: &[u8], hide_first: bool) -> String {
    cards
        .iter()
        .enumerate()
        .map(|(index, &card)| {
            if hide_first && index == 0 {
                "[?]".to_string()
            } else {
                format!("[{}]", blackjack_card_label(card))
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

fn draw_blackjack(
    state: &AppState,
    name: &str,
    player: &[u8],
    dealer: &[u8],
    standing: bool,
    message: &str,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        rows / 2 - 8,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 - 5,
        &format!(
            "Dealer: {}   {}",
            blackjack_hand_text(dealer, !standing),
            if standing {
                format!("({})", blackjack_value(dealer))
            } else {
                "(hidden)".to_string()
            }
        ),
        &theme,
        Role::Danger,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 - 1,
        &format!(
            "Player: {}   ({})",
            blackjack_hand_text(player, false),
            blackjack_value(player)
        ),
        &theme,
        Role::Success,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 3,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 6,
        &format!("Score {score}   Space/Enter hit   Right/Down stand   Q menu"),
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_battleship(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 58, name) {
        return;
    }
    loop {
        let ships = make_battleship_fleet(state);
        let mut shots = HashSet::new();
        let mut cursor = (0usize, 0usize);
        let mut torpedoes = match state.difficulty_index {
            0 => 36,
            1 => 30,
            _ => 25,
        };
        let mut message = "Find every ship cell.".to_string();
        let mut won = false;
        while torpedoes > 0 && !won {
            let hits = shots.iter().filter(|point| ships.contains(point)).count();
            draw_battleship(
                state, name, &ships, &shots, cursor, torpedoes, hits, &message,
            );
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor.1 > 0 => cursor.1 -= 1,
                    Key::Down | Key::Char('s') if cursor.1 < 7 => cursor.1 += 1,
                    Key::Left | Key::Char('a') if cursor.0 > 0 => cursor.0 -= 1,
                    Key::Right | Key::Char('d') if cursor.0 < 7 => cursor.0 += 1,
                    Key::Enter | Key::Space => {
                        if shots.insert(cursor) {
                            torpedoes -= 1;
                            if ships.contains(&cursor) {
                                message = "Hit.".to_string();
                                play_sound(state, "score");
                            } else {
                                let nearby = ships
                                    .iter()
                                    .filter(|&&(x, y)| {
                                        (x as i32 - cursor.0 as i32).abs() <= 1
                                            && (y as i32 - cursor.1 as i32).abs() <= 1
                                    })
                                    .count();
                                message = format!("Miss. Radar pings nearby: {nearby}.");
                                play_sound(state, "wall");
                            }
                            won = ships.iter().all(|point| shots.contains(point));
                        } else {
                            message = "Already fired there.".to_string();
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let hits = shots.iter().filter(|point| ships.contains(point)).count() as u32;
        let score = if won {
            700 + torpedoes as u32 * 12
        } else {
            hits * 40
        };
        record_score(state, name, score);
        let result = if won {
            "Fleet sunk.".to_string()
        } else {
            "Out of torpedoes.".to_string()
        };
        if !wait_menu(state, name, &[result, format!("Score: {score}")], true) {
            return;
        }
    }
}

fn make_battleship_fleet(state: &mut AppState) -> HashSet<(usize, usize)> {
    let mut ships = HashSet::new();
    for length in [4usize, 3, 3, 2, 2] {
        for _ in 0..200 {
            let horizontal = state.rng.chance(1, 2);
            let x = state.rng.usize(if horizontal { 9 - length } else { 8 });
            let y = state.rng.usize(if horizontal { 8 } else { 9 - length });
            let cells: Vec<_> = (0..length)
                .map(|offset| {
                    if horizontal {
                        (x + offset, y)
                    } else {
                        (x, y + offset)
                    }
                })
                .collect();
            if cells.iter().all(|cell| !ships.contains(cell)) {
                for cell in cells {
                    ships.insert(cell);
                }
                break;
            }
        }
    }
    ships
}

fn draw_battleship(
    state: &AppState,
    name: &str,
    ships: &HashSet<(usize, usize)>,
    shots: &HashSet<(usize, usize)>,
    cursor: (usize, usize),
    torpedoes: i32,
    hits: usize,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Hits {hits}/{}   Torpedoes {torpedoes}", ships.len()),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let top = rows / 2 - 8;
    let left = cols / 2 - 16;
    for y in 0..8 {
        for x in 0..8 {
            let point = (x, y);
            let fired = shots.contains(&point);
            let text = if fired && ships.contains(&point) {
                "[X]"
            } else if fired {
                "[.]"
            } else {
                "[ ]"
            };
            if point == cursor {
                put_inv(
                    &mut buf,
                    top + y * 2,
                    left + x * 4,
                    text,
                    &theme,
                    Role::Highlight,
                );
            } else {
                put(
                    &mut buf,
                    top + y * 2,
                    left + x * 4,
                    text,
                    &theme,
                    if fired && ships.contains(&point) {
                        Role::Danger
                    } else if fired {
                        Role::Muted
                    } else {
                        Role::Normal
                    },
                    fired,
                );
            }
        }
    }
    center(
        &mut buf,
        top + 18,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    center(
        &mut buf,
        top + 20,
        "Move cursor, Space fires, radar reports adjacent ship cells.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_tower_stack(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    loop {
        let board_w = 32i32;
        let layers_goal = 13usize;
        let mut settled: Vec<(i32, i32)> = Vec::new();
        let mut block_x = 0i32;
        let mut block_w = 11i32;
        let mut dir = 1i32;
        let mut score = 0u32;
        let mut failed = false;
        while settled.len() < layers_goal && !failed {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Enter | Key::Space => {
                        if let Some(&(prev_x, prev_w)) = settled.last() {
                            let start = block_x.max(prev_x);
                            let end = (block_x + block_w).min(prev_x + prev_w);
                            if end <= start {
                                failed = true;
                                play_sound(state, "alert");
                            } else {
                                block_x = start;
                                block_w = end - start;
                                settled.push((block_x, block_w));
                                score += 40 + block_w as u32 * 4;
                                play_sound(state, "score");
                            }
                        } else {
                            settled.push((block_x, block_w));
                            score += 40 + block_w as u32 * 4;
                            play_sound(state, "score");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if !failed && settled.len() < layers_goal {
                block_x += dir;
                if block_x <= 0 || block_x + block_w >= board_w {
                    dir = -dir;
                    block_x = block_x.clamp(0, board_w - block_w);
                }
            }
            draw_tower_stack(
                state,
                name,
                board_w,
                layers_goal,
                &settled,
                block_x,
                block_w,
                score,
            );
            let tick = (state.difficulty().tick_ms as f64 * 0.55) as u64;
            sleep_frame(frame, tick.max(22));
        }
        if settled.len() >= layers_goal {
            score += 500;
        }
        record_score(state, name, score);
        let result = if failed {
            "The stack slipped.".to_string()
        } else {
            "Tower complete.".to_string()
        };
        if !wait_menu(state, name, &[result, format!("Score: {score}")], true) {
            return;
        }
    }
}

fn draw_tower_stack(
    state: &AppState,
    name: &str,
    board_w: i32,
    layers_goal: usize,
    settled: &[(i32, i32)],
    block_x: i32,
    block_w: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - layers_goal / 2 - 2;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!(
            "Score {score}   Layer {}/{}   Space locks the moving block",
            settled.len() + 1,
            layers_goal
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        layers_goal + 4,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for (index, &(x, w)) in settled.iter().enumerate() {
        let y = top + layers_goal - index;
        put(
            &mut buf,
            y,
            left + x as usize,
            &"#".repeat(w as usize),
            &theme,
            Role::Success,
            true,
        );
    }
    if settled.len() < layers_goal {
        let y = top + layers_goal - settled.len();
        put(
            &mut buf,
            y,
            left + block_x as usize,
            &"=".repeat(block_w as usize),
            &theme,
            Role::Highlight,
            true,
        );
    }
    flush(&buf);
}

fn game_lights_out(state: &mut AppState, name: &str) {
    loop {
        let mut board = [false; 25];
        let toggles = match state.difficulty_index {
            0 => 9,
            1 => 13,
            _ => 17,
        };
        for _ in 0..toggles {
            let index = state.rng.usize(25);
            lights_toggle(&mut board, index % 5, index / 5);
        }
        let mut cursor = 12usize;
        let mut moves = 0u32;
        while board.iter().any(|&on| on) {
            draw_lights_out(state, name, &board, cursor, moves);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 5 => cursor -= 5,
                    Key::Down | Key::Char('s') if cursor < 20 => cursor += 5,
                    Key::Left | Key::Char('a') if cursor % 5 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 5 < 4 => cursor += 1,
                    Key::Enter | Key::Space => {
                        lights_toggle(&mut board, cursor % 5, cursor / 5);
                        moves += 1;
                        play_sound(state, "wall");
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let score = 650u32.saturating_sub(moves * 18);
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[
                format!("All lights out. Score: {score}"),
                format!("Moves: {moves}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn lights_toggle(board: &mut [bool; 25], x: usize, y: usize) {
    for (dx, dy) in [(0i32, 0i32), (1, 0), (-1, 0), (0, 1), (0, -1)] {
        let nx = x as i32 + dx;
        let ny = y as i32 + dy;
        if (0..5).contains(&nx) && (0..5).contains(&ny) {
            let index = ny as usize * 5 + nx as usize;
            board[index] = !board[index];
        }
    }
}

fn draw_lights_out(state: &AppState, name: &str, board: &[bool; 25], cursor: usize, moves: u32) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Moves {moves}   Space toggles a plus shape   Clear every light"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let top = rows / 2 - 5;
    let left = cols / 2 - 10;
    for y in 0..5 {
        for x in 0..5 {
            let index = y * 5 + x;
            let text = if board[index] { "[*]" } else { "[.]" };
            if index == cursor {
                put_inv(
                    &mut buf,
                    top + y * 2,
                    left + x * 4,
                    text,
                    &theme,
                    Role::Highlight,
                );
            } else {
                put(
                    &mut buf,
                    top + y * 2,
                    left + x * 4,
                    text,
                    &theme,
                    if board[index] {
                        Role::Success
                    } else {
                        Role::Muted
                    },
                    board[index],
                );
            }
        }
    }
    flush(&buf);
}

fn game_domino_chain(state: &mut AppState, name: &str) {
    loop {
        let mut hand: Vec<(u8, u8)> = (0..7)
            .map(|_| (state.rng.range(0, 6) as u8, state.rng.range(0, 6) as u8))
            .collect();
        let mut open = state.rng.range(0, 6) as u8;
        let mut cursor = 0usize;
        let mut score = 0u32;
        let mut plays = 0u32;
        let mut message = "Choose a domino matching the open end.".to_string();
        while !hand.is_empty() && hand.iter().any(|&(a, b)| a == open || b == open) {
            draw_domino_chain(state, name, &hand, open, cursor, score, &message);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Left | Key::Char('a') if cursor > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor + 1 < hand.len() => cursor += 1,
                    Key::Enter | Key::Space => {
                        let (a, b) = hand[cursor];
                        if a == open || b == open {
                            open = if a == open { b } else { a };
                            score += 40 + (a as u32 + b as u32) * 3;
                            plays += 1;
                            hand.remove(cursor);
                            if cursor >= hand.len() && cursor > 0 {
                                cursor -= 1;
                            }
                            message = format!("Chain length {plays}. Open end is {open}.");
                            play_sound(state, "score");
                        } else {
                            message = format!("Needs a {open} on either side.");
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        if hand.is_empty() {
            score += 300;
        }
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Chain ended. Score: {score}"),
                format!("Played: {plays}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_domino_chain(
    state: &AppState,
    name: &str,
    hand: &[(u8, u8)],
    open: u8,
    cursor: usize,
    score: u32,
    message: &str,
) {
    let (_, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        4,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        7,
        &format!("Open end: {open}   Score {score}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let row = hand
        .iter()
        .enumerate()
        .map(|(index, &(a, b))| {
            if index == cursor {
                format!(">{{{a}|{b}}}<")
            } else {
                format!(" [{a}|{b}] ")
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    center(&mut buf, 11, &row, &theme, Role::Success, true, cols);
    center(&mut buf, 15, message, &theme, Role::Secondary, true, cols);
    center(
        &mut buf,
        18,
        "A/D choose   Space plays matching domino",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_slide_puzzle(state: &mut AppState, name: &str) {
    loop {
        let mut tiles = [1u8, 2, 3, 4, 5, 6, 7, 8, 0];
        let mut blank = 8usize;
        for _ in 0..(80 + state.difficulty_index * 40) {
            let moves = slide_neighbors(blank);
            let next = moves[state.rng.usize(moves.len())];
            tiles.swap(blank, next);
            blank = next;
        }
        let mut moves_count = 0u32;
        while tiles != [1, 2, 3, 4, 5, 6, 7, 8, 0] {
            draw_slide_puzzle(state, name, &tiles, blank, moves_count);
            if let Some(key) = wait_for_key() {
                let target = match key {
                    Key::Up | Key::Char('w') if blank < 6 => Some(blank + 3),
                    Key::Down | Key::Char('s') if blank >= 3 => Some(blank - 3),
                    Key::Left | Key::Char('a') if blank % 3 < 2 => Some(blank + 1),
                    Key::Right | Key::Char('d') if blank % 3 > 0 => Some(blank - 1),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                        None
                    }
                    _ if is_quit(key) => return,
                    _ => None,
                };
                if let Some(target) = target {
                    tiles.swap(blank, target);
                    blank = target;
                    moves_count += 1;
                    play_sound(state, "wall");
                }
            }
        }
        let score = 900u32.saturating_sub(moves_count * 8);
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Solved. Score: {score}"),
                format!("Moves: {moves_count}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn slide_neighbors(blank: usize) -> Vec<usize> {
    let mut out = Vec::new();
    let x = blank % 3;
    let y = blank / 3;
    if x > 0 {
        out.push(blank - 1);
    }
    if x < 2 {
        out.push(blank + 1);
    }
    if y > 0 {
        out.push(blank - 3);
    }
    if y < 2 {
        out.push(blank + 3);
    }
    out
}

fn draw_slide_puzzle(
    state: &AppState,
    name: &str,
    tiles: &[u8; 9],
    blank: usize,
    moves_count: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Moves {moves_count}   Move tiles into the blank space"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let top = rows / 2 - 4;
    let left = cols / 2 - 9;
    for y in 0..3 {
        for x in 0..3 {
            let index = y * 3 + x;
            let text = if tiles[index] == 0 {
                "     ".to_string()
            } else {
                format!(" [{}] ", tiles[index])
            };
            if index == blank {
                put_inv(
                    &mut buf,
                    top + y * 3,
                    left + x * 6,
                    &text,
                    &theme,
                    Role::Highlight,
                );
            } else {
                put(
                    &mut buf,
                    top + y * 3,
                    left + x * 6,
                    &text,
                    &theme,
                    Role::Success,
                    true,
                );
            }
        }
    }
    flush(&buf);
}

fn game_mini_golf(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    loop {
        let (w, h) = (38i32, 16i32);
        let start = (2, h - 2);
        let hole = (w - 3, 1);
        let walls = make_golf_course(state, w, h);
        let mut ball = start;
        let mut aim = (1, 0);
        let mut shots = 0u32;
        let max_shots = match state.difficulty_index {
            0 => 18,
            1 => 15,
            _ => 12,
        };
        let mut won = false;
        while shots < max_shots && !won {
            draw_mini_golf(state, name, w, h, ball, hole, &walls, aim, shots, max_shots);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') => aim = (0, -1),
                    Key::Down | Key::Char('s') => aim = (0, 1),
                    Key::Left | Key::Char('a') => aim = (-1, 0),
                    Key::Right | Key::Char('d') => aim = (1, 0),
                    Key::Enter | Key::Space => {
                        shots += 1;
                        for _ in 0..w {
                            let next = (ball.0 + aim.0, ball.1 + aim.1);
                            if next == hole {
                                ball = next;
                                won = true;
                                play_sound(state, "score");
                                break;
                            }
                            if next.0 <= 0
                                || next.0 >= w - 1
                                || next.1 <= 0
                                || next.1 >= h - 1
                                || walls.contains(&next)
                            {
                                play_sound(state, "wall");
                                break;
                            }
                            ball = next;
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let score = if won {
            600u32.saturating_sub(shots * 25)
        } else {
            25
        };
        record_score(state, name, score);
        let result = if won {
            format!("Holed out in {shots}. Score: {score}")
        } else {
            format!("Out of strokes. Score: {score}")
        };
        if !wait_menu(state, name, &[result], true) {
            return;
        }
    }
}

fn make_golf_course(state: &mut AppState, w: i32, h: i32) -> HashSet<(i32, i32)> {
    let mut walls = HashSet::new();
    for x in [w / 4, w / 2, w * 3 / 4] {
        let gap = state.rng.range(3, h - 4);
        for y in 2..h - 1 {
            if (y - gap).abs() > 1 {
                walls.insert((x, y));
            }
        }
    }
    walls
}

fn draw_mini_golf(
    state: &AppState,
    name: &str,
    w: i32,
    h: i32,
    ball: (i32, i32),
    hole: (i32, i32),
    walls: &HashSet<(i32, i32)>,
    aim: (i32, i32),
    shots: u32,
    max_shots: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - h as usize / 2 + 1;
    let left = cols / 2 - w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Shots {shots}/{max_shots}   Aim WASD/arrows   Space putts"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        h as usize + 2,
        w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in walls {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "#",
            &theme,
            Role::Muted,
            false,
        );
    }
    put(
        &mut buf,
        top + hole.1 as usize,
        left + hole.0 as usize,
        "O",
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + ball.1 as usize,
        left + ball.0 as usize,
        "o",
        &theme,
        Role::Highlight,
        true,
    );
    let arrow = match aim {
        (0, -1) => "^",
        (0, 1) => "v",
        (-1, 0) => "<",
        _ => ">",
    };
    put(
        &mut buf,
        top + ball.1.saturating_sub(1) as usize,
        left + ball.0 as usize,
        arrow,
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_darts(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 58, name) {
        return;
    }
    loop {
        let mut cursor = (0i32, 0i32);
        let mut wind = (state.rng.range(-1, 1), state.rng.range(-1, 1));
        let mut throws = 0u32;
        let max_throws = 9u32;
        let mut score = 0u32;
        while throws < max_throws {
            draw_darts(state, name, cursor, wind, throws, max_throws, score);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') => cursor.1 = (cursor.1 - 1).max(-5),
                    Key::Down | Key::Char('s') => cursor.1 = (cursor.1 + 1).min(5),
                    Key::Left | Key::Char('a') => cursor.0 = (cursor.0 - 1).max(-10),
                    Key::Right | Key::Char('d') => cursor.0 = (cursor.0 + 1).min(10),
                    Key::Enter | Key::Space => {
                        let hit = (
                            (cursor.0 + wind.0).clamp(-10, 10),
                            (cursor.1 + wind.1).clamp(-5, 5),
                        );
                        let dist = hit.0 * hit.0 + hit.1 * hit.1 * 4;
                        let points = if dist <= 1 {
                            50
                        } else if dist <= 9 {
                            25
                        } else if dist <= 36 {
                            10
                        } else if dist <= 81 {
                            5
                        } else {
                            0
                        };
                        score += points;
                        throws += 1;
                        wind = (state.rng.range(-1, 1), state.rng.range(-1, 1));
                        if points > 0 {
                            play_sound(state, "score");
                        } else {
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        record_score(state, name, score);
        if !wait_menu(state, name, &[format!("Final score: {score}")], true) {
            return;
        }
    }
}

fn draw_darts(
    state: &AppState,
    name: &str,
    cursor: (i32, i32),
    wind: (i32, i32),
    throws: u32,
    max_throws: u32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 7;
    let left = cols / 2 - 12;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!(
            "Score {score}   Throw {throws}/{max_throws}   Wind {},{}",
            wind.0, wind.1
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for y in -5i32..=5 {
        for x in -10i32..=10 {
            let dist = x * x + y * y * 4;
            let ch = if (x, y) == cursor {
                "+"
            } else if dist <= 1 {
                "O"
            } else if dist <= 9 {
                "o"
            } else if dist <= 36 {
                "."
            } else if dist <= 81 {
                "`"
            } else {
                " "
            };
            let role = if (x, y) == cursor {
                Role::Highlight
            } else if dist <= 9 {
                Role::Success
            } else if dist <= 81 {
                Role::Muted
            } else {
                Role::Normal
            };
            put(
                &mut buf,
                (top as i32 + y + 5) as usize,
                (left as i32 + x + 10) as usize,
                ch,
                &theme,
                role,
                dist <= 9 || (x, y) == cursor,
            );
        }
    }
    center(
        &mut buf,
        top + 13,
        "Move reticle, Space throws. Wind nudges the dart after release.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_mancala(state: &mut AppState, name: &str) {
    if !require_size(state, 18, 60, name) {
        return;
    }
    loop {
        let stones = match state.difficulty_index {
            0 => 3,
            1 => 4,
            _ => 5,
        };
        let mut player = [stones; 6];
        let mut cpu = [stones; 6];
        let mut player_store = 0u8;
        let mut cpu_store = 0u8;
        let mut cursor = 2usize;
        let mut message = "Choose one of your pits and sow.".to_string();

        while !mancala_empty(&player) && !mancala_empty(&cpu) {
            draw_mancala(
                state,
                name,
                &player,
                &cpu,
                player_store,
                cpu_store,
                cursor,
                &message,
            );
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Left | Key::Char('a') if cursor > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor < 5 => cursor += 1,
                    Key::Enter | Key::Space => {
                        if player[cursor] == 0 {
                            message = "That pit is empty.".to_string();
                            play_sound(state, "wall");
                            continue;
                        }
                        let extra =
                            mancala_sow_player(&mut player, &mut cpu, &mut player_store, cursor);
                        play_sound(state, "score");
                        if extra {
                            message = "Last stone landed in your store. Go again.".to_string();
                            continue;
                        }
                        while !mancala_empty(&cpu) {
                            let cpu_choice = mancala_cpu_choice(&cpu);
                            let cpu_extra =
                                mancala_sow_cpu(&mut player, &mut cpu, &mut cpu_store, cpu_choice);
                            if !cpu_extra {
                                break;
                            }
                        }
                        message = "Your move.".to_string();
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }

        player_store += player.iter().sum::<u8>();
        cpu_store += cpu.iter().sum::<u8>();
        let score = if player_store >= cpu_store {
            500 + player_store as u32 * 20
        } else {
            player_store as u32 * 20
        };
        record_score(state, name, score);
        let result = if player_store > cpu_store {
            format!("You won {player_store}-{cpu_store}. Score: {score}")
        } else if player_store == cpu_store {
            format!("Tie game {player_store}-{cpu_store}. Score: {score}")
        } else {
            format!("CPU won {cpu_store}-{player_store}. Score: {score}")
        };
        if !wait_menu(state, name, &[result], true) {
            return;
        }
    }
}

fn mancala_empty(side: &[u8; 6]) -> bool {
    side.iter().all(|&stones| stones == 0)
}

fn mancala_cpu_choice(cpu: &[u8; 6]) -> usize {
    let mut choice = 0usize;
    let mut best = 0u8;
    for (index, &stones) in cpu.iter().enumerate() {
        if stones > best {
            best = stones;
            choice = index;
        }
    }
    choice
}

fn mancala_sow_player(player: &mut [u8; 6], cpu: &mut [u8; 6], store: &mut u8, pit: usize) -> bool {
    let mut stones = player[pit];
    player[pit] = 0;
    let mut pos = pit;
    while stones > 0 {
        pos = (pos + 1) % 13;
        match pos {
            0..=5 => player[pos] += 1,
            6 => *store += 1,
            _ => cpu[12 - pos] += 1,
        }
        stones -= 1;
    }
    if pos < 6 && player[pos] == 1 {
        let opposite = 5 - pos;
        if cpu[opposite] > 0 {
            *store += cpu[opposite] + 1;
            cpu[opposite] = 0;
            player[pos] = 0;
        }
    }
    pos == 6
}

fn mancala_sow_cpu(player: &mut [u8; 6], cpu: &mut [u8; 6], store: &mut u8, pit: usize) -> bool {
    let mut stones = cpu[pit];
    cpu[pit] = 0;
    let mut pos = pit;
    while stones > 0 {
        pos = (pos + 1) % 13;
        match pos {
            0..=5 => cpu[pos] += 1,
            6 => *store += 1,
            _ => player[12 - pos] += 1,
        }
        stones -= 1;
    }
    if pos < 6 && cpu[pos] == 1 {
        let opposite = 5 - pos;
        if player[opposite] > 0 {
            *store += player[opposite] + 1;
            player[opposite] = 0;
            cpu[pos] = 0;
        }
    }
    pos == 6
}

fn draw_mancala(
    state: &AppState,
    name: &str,
    player: &[u8; 6],
    cpu: &[u8; 6],
    player_store: u8,
    cpu_store: u8,
    cursor: usize,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        "A/D choose pit   Space sows   Captures use the opposite pit",
        &theme,
        Role::Accent,
        false,
        cols,
    );
    let top = rows / 2 - 4;
    let left = cols / 2 - 23;
    put(
        &mut buf,
        top,
        left,
        &format!("CPU store {:02}", cpu_store),
        &theme,
        Role::Danger,
        true,
    );
    put(
        &mut buf,
        top,
        left + 33,
        &format!("Your store {:02}", player_store),
        &theme,
        Role::Success,
        true,
    );
    for i in 0..6 {
        let x = left + 10 + i * 5;
        put(
            &mut buf,
            top + 2,
            x,
            &format!("[{}]", cpu[5 - i]),
            &theme,
            Role::Danger,
            true,
        );
        let text = format!("[{}]", player[i]);
        if i == cursor {
            put_inv(&mut buf, top + 5, x, &text, &theme, Role::Highlight);
        } else {
            put(&mut buf, top + 5, x, &text, &theme, Role::Success, true);
        }
    }
    center(
        &mut buf,
        top + 8,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_mini_sudoku(state: &mut AppState, name: &str) {
    if !require_size(state, 18, 50, name) {
        return;
    }
    loop {
        let solution = mini_sudoku_solution(state.rng.usize(4) as u8);
        let mut board = solution;
        let mut fixed = [true; 16];
        let blanks = match state.difficulty_index {
            0 => 6,
            1 => 8,
            _ => 10,
        };
        let mut removed = 0;
        while removed < blanks {
            let index = state.rng.usize(16);
            if fixed[index] {
                fixed[index] = false;
                board[index] = 0;
                removed += 1;
            }
        }
        let mut cursor = 0usize;
        let mut moves_count = 0u32;
        let mut message = "Fill blanks with 1-4.".to_string();
        loop {
            let won = board == solution;
            draw_mini_sudoku(state, name, &board, &solution, &fixed, cursor, &message);
            if won {
                let score = 800u32.saturating_sub(moves_count * 12);
                record_score(state, name, score);
                if !wait_menu(
                    state,
                    name,
                    &[
                        format!("Solved. Score: {score}"),
                        format!("Moves: {moves_count}"),
                    ],
                    true,
                ) {
                    return;
                }
                break;
            }
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 4 => cursor -= 4,
                    Key::Down | Key::Char('s') if cursor < 12 => cursor += 4,
                    Key::Left | Key::Char('a') if cursor % 4 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 4 < 3 => cursor += 1,
                    Key::Char(ch @ '1'..='4') if !fixed[cursor] => {
                        board[cursor] = ch as u8 - b'0';
                        moves_count += 1;
                        if board[cursor] == solution[cursor] {
                            message = "Correct.".to_string();
                            play_sound(state, "score");
                        } else {
                            message = "That breaks the puzzle.".to_string();
                            play_sound(state, "wall");
                        }
                    }
                    Key::Backspace | Key::Char('0') if !fixed[cursor] => {
                        board[cursor] = 0;
                        message = "Cell cleared.".to_string();
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
    }
}

fn mini_sudoku_solution(shift: u8) -> [u8; 16] {
    let base = [1, 2, 3, 4, 3, 4, 1, 2, 2, 1, 4, 3, 4, 3, 2, 1];
    let mut solution = [0u8; 16];
    for (index, value) in base.iter().enumerate() {
        solution[index] = ((*value + shift - 1) % 4) + 1;
    }
    solution
}

fn draw_mini_sudoku(
    state: &AppState,
    name: &str,
    board: &[u8; 16],
    solution: &[u8; 16],
    fixed: &[bool; 16],
    cursor: usize,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 5;
    let left = cols / 2 - 10;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        "Move cursor, press 1-4, Backspace clears editable cells.",
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for y in 0..4 {
        for x in 0..4 {
            let index = y * 4 + x;
            let value = board[index];
            let text = if value == 0 {
                " . ".to_string()
            } else {
                format!(" {value} ")
            };
            let role = if fixed[index] {
                Role::Muted
            } else if value != 0 && value != solution[index] {
                Role::Danger
            } else {
                Role::Success
            };
            if index == cursor {
                put_inv(&mut buf, top + y * 2, left + x * 5, &text, &theme, role);
            } else {
                put(
                    &mut buf,
                    top + y * 2,
                    left + x * 5,
                    &text,
                    &theme,
                    role,
                    true,
                );
            }
        }
    }
    center(
        &mut buf,
        top + 10,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_reversi(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 60, name) {
        return;
    }
    loop {
        let mut board = [' '; 36];
        board[2 * 6 + 2] = 'O';
        board[3 * 6 + 3] = 'O';
        board[2 * 6 + 3] = 'X';
        board[3 * 6 + 2] = 'X';
        let mut cursor = 2 * 6 + 1;
        let mut message = "You are X. Flip CPU lines.".to_string();

        while reversi_has_move(&board, 'X', 'O') || reversi_has_move(&board, 'O', 'X') {
            if !reversi_has_move(&board, 'X', 'O') {
                reversi_cpu_move(&mut board);
                message = "No legal player move, CPU played.".to_string();
                continue;
            }
            draw_reversi(state, name, &board, cursor, &message);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 6 => cursor -= 6,
                    Key::Down | Key::Char('s') if cursor < 30 => cursor += 6,
                    Key::Left | Key::Char('a') if cursor % 6 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 6 < 5 => cursor += 1,
                    Key::Enter | Key::Space => {
                        let flips = reversi_flips(&board, cursor % 6, cursor / 6, 'X', 'O');
                        if flips.is_empty() {
                            message = "Legal moves must trap CPU discs in a line.".to_string();
                            play_sound(state, "wall");
                        } else {
                            board[cursor] = 'X';
                            for index in flips {
                                board[index] = 'X';
                            }
                            play_sound(state, "score");
                            if reversi_has_move(&board, 'O', 'X') {
                                reversi_cpu_move(&mut board);
                            }
                            message = "Your move.".to_string();
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }

        draw_reversi(state, name, &board, cursor, "Board complete.");
        let player_count = board.iter().filter(|&&cell| cell == 'X').count() as u32;
        let cpu_count = board.iter().filter(|&&cell| cell == 'O').count() as u32;
        let score = if player_count >= cpu_count {
            500 + player_count * 20
        } else {
            player_count * 20
        };
        record_score(state, name, score);
        let result = if player_count > cpu_count {
            format!("You won {player_count}-{cpu_count}. Score: {score}")
        } else if player_count == cpu_count {
            format!("Draw {player_count}-{cpu_count}. Score: {score}")
        } else {
            format!("CPU won {cpu_count}-{player_count}. Score: {score}")
        };
        if !wait_menu(state, name, &[result], true) {
            return;
        }
    }
}

fn reversi_has_move(board: &[char; 36], me: char, other: char) -> bool {
    (0..36).any(|index| !reversi_flips(board, index % 6, index / 6, me, other).is_empty())
}

fn reversi_cpu_move(board: &mut [char; 36]) {
    let mut best_index = None;
    let mut best_flips = Vec::new();
    for index in 0..36 {
        let flips = reversi_flips(board, index % 6, index / 6, 'O', 'X');
        if flips.len() > best_flips.len() {
            best_index = Some(index);
            best_flips = flips;
        }
    }
    if let Some(index) = best_index {
        board[index] = 'O';
        for flip in best_flips {
            board[flip] = 'O';
        }
    }
}

fn reversi_flips(board: &[char; 36], x: usize, y: usize, me: char, other: char) -> Vec<usize> {
    if board[y * 6 + x] != ' ' {
        return Vec::new();
    }
    let mut flips = Vec::new();
    for (dx, dy) in [
        (-1, -1),
        (0, -1),
        (1, -1),
        (-1, 0),
        (1, 0),
        (-1, 1),
        (0, 1),
        (1, 1),
    ] {
        let mut path = Vec::new();
        let mut nx = x as i32 + dx;
        let mut ny = y as i32 + dy;
        while (0..6).contains(&nx) && (0..6).contains(&ny) {
            let index = ny as usize * 6 + nx as usize;
            if board[index] == other {
                path.push(index);
            } else if board[index] == me {
                if !path.is_empty() {
                    flips.extend(path);
                }
                break;
            } else {
                break;
            }
            nx += dx;
            ny += dy;
        }
    }
    flips
}

fn draw_reversi(state: &AppState, name: &str, board: &[char; 36], cursor: usize, message: &str) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 7;
    let left = cols / 2 - 14;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    let player_count = board.iter().filter(|&&cell| cell == 'X').count();
    let cpu_count = board.iter().filter(|&&cell| cell == 'O').count();
    center(
        &mut buf,
        3,
        &format!("X {player_count}   O {cpu_count}   Space places a disc"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for y in 0..6 {
        for x in 0..6 {
            let index = y * 6 + x;
            let cell = board[index];
            let text = match cell {
                'X' => "[X]",
                'O' => "[O]",
                _ => "[ ]",
            };
            let role = match cell {
                'X' => Role::Success,
                'O' => Role::Danger,
                _ => Role::Muted,
            };
            if index == cursor {
                put_inv(
                    &mut buf,
                    top + y * 2,
                    left + x * 5,
                    text,
                    &theme,
                    Role::Highlight,
                );
            } else {
                put(
                    &mut buf,
                    top + y * 2,
                    left + x * 5,
                    text,
                    &theme,
                    role,
                    true,
                );
            }
        }
    }
    center(
        &mut buf,
        top + 14,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_bowling(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    loop {
        let mut frame = 1u32;
        let mut total = 0u32;
        let mut aim = 3i32;
        let mut power = 7i32;
        let mut message = "A/D aim lane, W/S power, Space rolls.".to_string();

        while frame <= 10 {
            let mut pins_left = 10i32;
            let mut roll = 1u32;
            while roll <= 2 && pins_left > 0 {
                draw_bowling(
                    state, name, frame, roll, aim, power, pins_left, total, &message,
                );
                if let Some(key) = wait_for_key() {
                    match key {
                        Key::Left | Key::Char('a') => aim = (aim - 1).max(0),
                        Key::Right | Key::Char('d') => aim = (aim + 1).min(6),
                        Key::Up | Key::Char('w') => power = (power + 1).min(10),
                        Key::Down | Key::Char('s') => power = (power - 1).max(1),
                        Key::Enter | Key::Space => {
                            let knocked = bowling_roll(state, aim, power, pins_left);
                            pins_left -= knocked;
                            total += knocked as u32;
                            if knocked == 10 {
                                total += 10;
                                message = "Strike bonus.".to_string();
                                play_sound(state, "score");
                                break;
                            }
                            if pins_left == 0 {
                                total += 5;
                                message = "Spare bonus.".to_string();
                                play_sound(state, "score");
                                break;
                            }
                            message = format!("{knocked} pins down, {pins_left} standing.");
                            play_sound(state, "wall");
                            roll += 1;
                        }
                        _ if is_pause(key) => {
                            if pause_screen(state).is_none() {
                                return;
                            }
                        }
                        _ if is_quit(key) => return,
                        _ => {}
                    }
                }
            }
            frame += 1;
        }
        let score = total * 8;
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Pins and bonuses: {total}"),
                format!("Score: {score}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn bowling_roll(state: &mut AppState, aim: i32, power: i32, pins_left: i32) -> i32 {
    let drift = state.rng.range(-1, 1);
    let lane_error = (aim + drift - 3).abs();
    let power_error = (power - 7).abs();
    let base = 10 - lane_error * 3 - power_error;
    base.clamp(0, pins_left)
}

fn draw_bowling(
    state: &AppState,
    name: &str,
    frame: u32,
    roll: u32,
    aim: i32,
    power: i32,
    pins_left: i32,
    total: u32,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 7;
    let left = cols / 2 - 16;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Frame {frame}/10 Roll {roll}   Total {total}   Power {power}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for y in 0..10 {
        put(&mut buf, top + y, left, "|", &theme, Role::Muted, false);
        put(
            &mut buf,
            top + y,
            left + 31,
            "|",
            &theme,
            Role::Muted,
            false,
        );
    }
    for pin in 0..pins_left {
        let x = left + 12 + (pin as usize % 5) * 2 + (pin as usize / 5);
        let y = top + pin as usize / 5;
        put(&mut buf, y, x, "^", &theme, Role::Danger, true);
    }
    for lane in 0..7 {
        let x = left + 4 + lane * 4;
        let role = if lane as i32 == aim {
            Role::Highlight
        } else {
            Role::Muted
        };
        put(&mut buf, top + 9, x, "o", &theme, role, lane as i32 == aim);
    }
    center(
        &mut buf,
        top + 12,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_skee_ball(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 60, name) {
        return;
    }
    loop {
        let mut lane = 2i32;
        let mut power = 6i32;
        let mut balls = 0u32;
        let mut score = 0u32;
        let max_balls = 9u32;
        let mut message = "A/D lane, W/S power, Space rolls.".to_string();
        while balls < max_balls {
            draw_skee_ball(state, name, lane, power, balls, max_balls, score, &message);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Left | Key::Char('a') => lane = (lane - 1).max(0),
                    Key::Right | Key::Char('d') => lane = (lane + 1).min(4),
                    Key::Up | Key::Char('w') => power = (power + 1).min(10),
                    Key::Down | Key::Char('s') => power = (power - 1).max(1),
                    Key::Enter | Key::Space => {
                        let drift = state.rng.range(-1, 1);
                        let final_lane = (lane + drift).clamp(0, 4);
                        let lane_error = (final_lane - 2).abs();
                        let power_error = (power - 7).abs();
                        let points = match lane_error + power_error {
                            0 => 100,
                            1 => 50,
                            2 => 30,
                            3 => 20,
                            _ => 10,
                        };
                        score += points;
                        balls += 1;
                        message = format!("Ball scored {points}. Drift was {drift}.");
                        if points >= 50 {
                            play_sound(state, "score");
                        } else {
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        record_score(state, name, score);
        if !wait_menu(state, name, &[format!("Final score: {score}")], true) {
            return;
        }
    }
}

fn draw_skee_ball(
    state: &AppState,
    name: &str,
    lane: i32,
    power: i32,
    balls: u32,
    max_balls: u32,
    score: u32,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 7;
    let left = cols / 2 - 15;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Score {score}   Ball {balls}/{max_balls}   Power {power}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for (row, label) in [" [100] ", "  [50] ", "  [30] ", "  [20] ", "  [10] "]
        .iter()
        .enumerate()
    {
        center(
            &mut buf,
            top + row * 2,
            label,
            &theme,
            if row == 0 {
                Role::Success
            } else {
                Role::Secondary
            },
            true,
            cols,
        );
    }
    for x in 0..5 {
        let text = if x == lane { " ^ " } else { " . " };
        let role = if x == lane {
            Role::Highlight
        } else {
            Role::Muted
        };
        put(
            &mut buf,
            top + 12,
            left + x as usize * 7,
            text,
            &theme,
            role,
            true,
        );
    }
    center(
        &mut buf,
        top + 15,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

fn game_keeper(state: &mut AppState, name: &str) {
    if !require_size(state, 22, 54, name) {
        return;
    }
    loop {
        let lanes = 7i32;
        let goal_y = 12i32;
        let target_saves = match state.difficulty_index {
            0 => 12,
            1 => 16,
            _ => 20,
        };
        let mut keeper = 3i32;
        let mut shot_lane = state.rng.range(0, lanes - 1);
        let mut shot_y = 0i32;
        let mut saves = 0u32;
        let mut misses = 0u32;
        let mut message = "A/D move keeper. Block the shot lane.".to_string();

        while misses < 5 && saves < target_saves {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => keeper = (keeper - 1).max(0),
                    Key::Right | Key::Char('d') => keeper = (keeper + 1).min(lanes - 1),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            draw_keeper(
                state,
                name,
                lanes,
                goal_y,
                keeper,
                shot_lane,
                shot_y,
                saves,
                misses,
                target_saves,
                &message,
            );
            shot_y += 1;
            if shot_y >= goal_y {
                if shot_lane == keeper {
                    saves += 1;
                    message = "Save.".to_string();
                    play_sound(state, "score");
                } else {
                    misses += 1;
                    message = "Goal conceded.".to_string();
                    play_sound(state, "alert");
                }
                shot_lane = state.rng.range(0, lanes - 1);
                shot_y = 0;
            }
            let tick = (state.difficulty().tick_ms as f64 * 1.25) as u64;
            sleep_frame(frame, tick.max(35));
        }

        let score = saves * 60 + if saves >= target_saves { 400 } else { 0 };
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Saves: {saves}   Misses: {misses}"),
                format!("Score: {score}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_keeper(
    state: &AppState,
    name: &str,
    lanes: i32,
    goal_y: i32,
    keeper: i32,
    shot_lane: i32,
    shot_y: i32,
    saves: u32,
    misses: u32,
    target_saves: u32,
    message: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 7;
    let left = cols / 2 - 16;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        1,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        3,
        &format!("Saves {saves}/{target_saves}   Misses {misses}/5"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for lane in 0..lanes {
        let x = left + lane as usize * 5;
        put(&mut buf, top, x, "|", &theme, Role::Muted, false);
        put(
            &mut buf,
            top + goal_y as usize,
            x,
            if lane == keeper { "[K]" } else { "___" },
            &theme,
            if lane == keeper {
                Role::Highlight
            } else {
                Role::Muted
            },
            lane == keeper,
        );
    }
    put(
        &mut buf,
        top + shot_y as usize,
        left + shot_lane as usize * 5,
        " o ",
        &theme,
        Role::Danger,
        true,
    );
    center(
        &mut buf,
        top + goal_y as usize + 3,
        message,
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    flush(&buf);
}

#[derive(Clone, Copy)]
struct QuestRules {
    mechanic: &'static str,
    maze: bool,
    item_count: usize,
    hazards_base: usize,
    hazards_per_difficulty: usize,
    ordered_count: usize,
    resource_start: i32,
    lives_enabled: bool,
    gated_items: bool,
    hazard_shift_steps: u32,
    dust_push_steps: u32,
    wrong_node_resets: bool,
}

fn quest_rules(kind: QuestKind) -> QuestRules {
    match kind {
        QuestKind::Checkmate => QuestRules {
            mechanic: "knight-leap-maze",
            maze: true,
            item_count: 0,
            hazards_base: 0,
            hazards_per_difficulty: 0,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Cipher => QuestRules {
            mechanic: "ordered-cipher-reset",
            maze: true,
            item_count: 0,
            hazards_base: 0,
            hazards_per_difficulty: 0,
            ordered_count: 5,
            resource_start: 0,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: true,
        },
        QuestKind::Marble => QuestRules {
            mechanic: "sliding-marble-stops",
            maze: true,
            item_count: 0,
            hazards_base: 0,
            hazards_per_difficulty: 0,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Quantum => QuestRules {
            mechanic: "shifting-hidden-hazards",
            maze: false,
            item_count: 0,
            hazards_base: 16,
            hazards_per_difficulty: 8,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 9,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Go => QuestRules {
            mechanic: "open-territory-claim",
            maze: false,
            item_count: 9,
            hazards_base: 0,
            hazards_per_difficulty: 0,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: true,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Pirate => QuestRules {
            mechanic: "treasure-with-patrol-lives",
            maze: true,
            item_count: 5,
            hazards_base: 5,
            hazards_per_difficulty: 2,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: true,
            gated_items: true,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Samurai => QuestRules {
            mechanic: "honor-trace-sentry-lives",
            maze: true,
            item_count: 0,
            hazards_base: 6,
            hazards_per_difficulty: 2,
            ordered_count: 5,
            resource_start: 0,
            lives_enabled: true,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Mars => QuestRules {
            mechanic: "battery-dust-hidden-field",
            maze: false,
            item_count: 0,
            hazards_base: 18,
            hazards_per_difficulty: 7,
            ordered_count: 0,
            resource_start: 70,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 13,
            wrong_node_resets: false,
        },
        QuestKind::DeepSea => QuestRules {
            mechanic: "pressure-sonar-oxygen-vents",
            maze: false,
            item_count: 4,
            hazards_base: 18,
            hazards_per_difficulty: 7,
            ordered_count: 0,
            resource_start: 85,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Volcano => QuestRules {
            mechanic: "rising-lava-relic-race",
            maze: true,
            item_count: 4,
            hazards_base: 0,
            hazards_per_difficulty: 0,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: true,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Jungle => QuestRules {
            mechanic: "visible-trap-relic-maze",
            maze: true,
            item_count: 5,
            hazards_base: 12,
            hazards_per_difficulty: 2,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: true,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Dragon => QuestRules {
            mechanic: "hot-lair-gold-heist",
            maze: true,
            item_count: 5,
            hazards_base: 8,
            hazards_per_difficulty: 3,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: true,
            gated_items: true,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
        QuestKind::Mirror => QuestRules {
            mechanic: "mirrored-sliding-maze",
            maze: true,
            item_count: 0,
            hazards_base: 0,
            hazards_per_difficulty: 0,
            ordered_count: 0,
            resource_start: 0,
            lives_enabled: false,
            gated_items: false,
            hazard_shift_steps: 0,
            dust_push_steps: 0,
            wrong_node_resets: false,
        },
    }
}

fn game_micro_quest(state: &mut AppState, name: &str, kind: QuestKind) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    loop {
        let rules = quest_rules(kind);
        let (w, h) = full_board(44, 15, 96, 32);
        let start = (1, h - 2);
        let goal = (w - 2, 1);
        let walls = if rules.maze {
            make_maze(state, w, h, start, goal)
        } else {
            HashSet::new()
        };
        let item_count = rules.item_count;
        let mut blocked = HashSet::from([start, goal]);
        let mut items = HashSet::new();
        for _ in 0..item_count {
            let point = quest_open_point(state, w, h, start, &walls, &blocked);
            blocked.insert(point);
            items.insert(point);
        }
        let hazard_count =
            rules.hazards_base + state.difficulty_index * rules.hazards_per_difficulty;
        let mut hazards = HashSet::new();
        for _ in 0..hazard_count {
            let point = quest_open_point(state, w, h, start, &walls, &blocked);
            blocked.insert(point);
            hazards.insert(point);
        }
        let ordered_count = rules.ordered_count;
        let mut nodes = Vec::new();
        for n in 1..=ordered_count {
            let point = quest_open_point(state, w, h, start, &walls, &blocked);
            blocked.insert(point);
            nodes.push((point.0, point.1, n as i32));
        }

        let mut player = start;
        let mut steps = 0u32;
        let mut next_node = 1i32;
        let mut resource = rules.resource_start;
        let mut lives = if rules.lives_enabled {
            state.starting_lives()
        } else {
            1
        };
        let mut lava_y = h - 1;
        let mut last_step_effect = 0u32;
        let mut score_bonus = 0u32;
        let mut status = format!("{} [{}]", quest_help(kind), rules.mechanic);
        let mut won = false;
        let mut lost = false;

        while !won && !lost {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                if is_pause(key) {
                    if pause_screen(state).is_none() {
                        return;
                    }
                    continue;
                }
                if is_quit(key) {
                    return;
                }
                let mut delta = match key {
                    Key::Up | Key::Char('w') => (0, -1),
                    Key::Down | Key::Char('s') => (0, 1),
                    Key::Left | Key::Char('a') => (-1, 0),
                    Key::Right | Key::Char('d') => (1, 0),
                    _ => (0, 0),
                };
                if matches!(kind, QuestKind::Mirror) {
                    delta.0 = -delta.0;
                }
                if matches!(kind, QuestKind::Checkmate) {
                    delta = match key {
                        Key::Up | Key::Char('w') => (1, -2),
                        Key::Down | Key::Char('s') => (-1, 2),
                        Key::Left | Key::Char('a') => (-2, -1),
                        Key::Right | Key::Char('d') => (2, 1),
                        _ => (0, 0),
                    };
                }
                if delta == (0, 0) {
                    continue;
                }
                let moved = if matches!(kind, QuestKind::Marble | QuestKind::Mirror) {
                    let mut any = false;
                    loop {
                        let next = (player.0 + delta.0, player.1 + delta.1);
                        if !quest_can_enter(next, w, h, &walls) {
                            break;
                        }
                        player = next;
                        steps += 1;
                        any = true;
                        if items.remove(&player) {
                            score_bonus += 40;
                            play_sound(state, "score");
                        }
                    }
                    any
                } else {
                    let next = (player.0 + delta.0, player.1 + delta.1);
                    if quest_can_enter(next, w, h, &walls) {
                        player = next;
                        steps += 1;
                        true
                    } else {
                        false
                    }
                };
                if moved {
                    play_sound(state, "wall");
                }
            }

            if items.remove(&player) {
                score_bonus += match kind {
                    QuestKind::Go => 30,
                    QuestKind::DeepSea => {
                        resource = (resource + 28).min(100);
                        20
                    }
                    QuestKind::Pirate | QuestKind::Dragon => 55,
                    _ => 40,
                };
                play_sound(state, "score");
            }
            if let Some(&(x, y, n)) = nodes
                .iter()
                .find(|&&(x, y, n)| (x, y) == player && n != next_node)
            {
                if rules.wrong_node_resets {
                    player = start;
                    steps += 5;
                    status = format!("Wrong cipher node {n}; route scrambled.");
                    play_sound(state, "alert");
                } else if matches!(kind, QuestKind::Samurai) {
                    lives -= 1;
                    player = start;
                    status = format!("Wrong honor mark at {x},{y}. Lives {lives}.");
                    play_sound(state, "alert");
                    if lives <= 0 {
                        lost = true;
                    }
                }
            }
            if let Some(pos) = nodes
                .iter()
                .position(|&(x, y, n)| (x, y) == player && n == next_node)
            {
                nodes.remove(pos);
                next_node += 1;
                score_bonus += 50;
                play_sound(state, "score");
            }

            if steps != last_step_effect {
                last_step_effect = steps;
                if rules.hazard_shift_steps > 0 && steps % rules.hazard_shift_steps == 0 {
                    if let Some(old) = hazards.iter().copied().next() {
                        hazards.remove(&old);
                        let replacement = quest_open_point(state, w, h, start, &walls, &blocked);
                        hazards.insert(replacement);
                    }
                }
                if rules.dust_push_steps > 0 && steps % rules.dust_push_steps == 0 {
                    let delta = if (steps / rules.dust_push_steps) % 2 == 0 {
                        (1, 0)
                    } else {
                        (-1, 0)
                    };
                    let pushed = (player.0 + delta.0, player.1 + delta.1);
                    if quest_can_enter(pushed, w, h, &walls) {
                        player = pushed;
                    }
                    resource -= 4 + state.difficulty_index as i32;
                }
            }
            if matches!(kind, QuestKind::Volcano) && steps > 0 && steps % 12 == 0 {
                lava_y = (lava_y - 1).max(2);
            }
            if matches!(kind, QuestKind::DeepSea) && steps > 0 {
                resource -= 1 + state.difficulty_index as i32;
            }
            if matches!(kind, QuestKind::Mars | QuestKind::Quantum) && hazards.contains(&player) {
                lost = true;
                status = "Scanner missed a hidden hazard.".to_string();
                play_sound(state, "alert");
            }
            if matches!(kind, QuestKind::DeepSea) && hazards.contains(&player) {
                resource -= 25;
                player = start;
                status = "Sonar hit a pressure mine; back to the trench mouth.".to_string();
                play_sound(state, "alert");
            }
            if matches!(
                kind,
                QuestKind::Dragon | QuestKind::Jungle | QuestKind::Pirate | QuestKind::Samurai
            ) && hazards.contains(&player)
            {
                if rules.lives_enabled {
                    lives -= 1;
                    player = start;
                    status = format!("Patrol hit. Lives {lives}.");
                    if lives <= 0 {
                        lost = true;
                    }
                } else {
                    lost = true;
                    status = "Caught by the lair traps.".to_string();
                }
                play_sound(state, "alert");
            }
            if matches!(kind, QuestKind::Volcano) && player.1 >= lava_y {
                lost = true;
                status = "The lava reached you.".to_string();
                play_sound(state, "alert");
            }
            if matches!(kind, QuestKind::Mars) && resource <= 0 {
                lost = true;
                status = "The rover battery died in the dust.".to_string();
                play_sound(state, "alert");
            }
            if matches!(kind, QuestKind::DeepSea) && resource <= 0 {
                lost = true;
                status = "Pressure crushed the run.".to_string();
                play_sound(state, "alert");
            }

            let nearby = hazards
                .iter()
                .filter(|&&(x, y)| (x - player.0).abs() <= 1 && (y - player.1).abs() <= 1)
                .count();
            if !lost {
                status = match kind {
                    QuestKind::Checkmate => {
                        "Knight-style moves only. Land on the king.".to_string()
                    }
                    QuestKind::Cipher | QuestKind::Samurai => {
                        format!("Trace node {next_node}, then reach the exit.")
                    }
                    QuestKind::Marble => "Slide until blocked; plan each roll.".to_string(),
                    QuestKind::Mirror => {
                        "Left and right are mirrored; sliding movement is active.".to_string()
                    }
                    QuestKind::Quantum | QuestKind::Mars => {
                        if matches!(kind, QuestKind::Mars) {
                            format!("Battery {resource}. Scanner ping: {nearby}. Dust pushes.")
                        } else {
                            format!("Scanner ping: {nearby}. Hazards blink every 9 steps.")
                        }
                    }
                    QuestKind::DeepSea => {
                        format!("Pressure {resource}. Sonar ping: {nearby}. Vents refill.")
                    }
                    QuestKind::Volcano => {
                        format!("Relics left: {}. Lava row: {lava_y}.", items.len())
                    }
                    QuestKind::Go => {
                        format!("Territory left: {}. Exit after claiming it.", items.len())
                    }
                    QuestKind::Pirate => {
                        format!("Treasure left: {}. Lives {lives}.", items.len())
                    }
                    QuestKind::Jungle => {
                        format!(
                            "Relics left: {}. Visible traps block greedy routes.",
                            items.len()
                        )
                    }
                    QuestKind::Dragon => {
                        format!(
                            "Gold left: {}. Lives {lives}. Avoid hot marks.",
                            items.len()
                        )
                    }
                };
            }

            let needs_items = rules.gated_items;
            let needs_nodes = ordered_count > 0;
            if player == goal
                && (!needs_items || items.is_empty())
                && (!needs_nodes || next_node > ordered_count as i32)
            {
                won = true;
                play_sound(state, "score");
            }

            draw_micro_quest(
                state, name, kind, w, h, player, goal, &walls, &items, &hazards, &nodes, lava_y,
                steps, &status,
            );
            sleep_frame(frame, 55);
        }

        let score = if won {
            900u32.saturating_sub(steps * 4) + score_bonus
        } else {
            score_bonus + steps
        };
        record_score(state, name, score);
        let result = if won {
            format!("Mission complete. Score: {score}")
        } else {
            format!("{status} Score: {score}")
        };
        if !wait_menu(state, name, &[result], true) {
            return;
        }
    }
}

fn quest_help(kind: QuestKind) -> &'static str {
    match kind {
        QuestKind::Checkmate => "Use chessy knight leaps to reach the king.",
        QuestKind::Cipher => "Trace cipher nodes in order; wrong nodes reset the route.",
        QuestKind::Marble => "Roll until blocked.",
        QuestKind::Quantum => "Use scanner pings; hidden hazards blink around.",
        QuestKind::Go => "Claim territory before exiting.",
        QuestKind::Pirate => "Collect treasure while patrols burn lives.",
        QuestKind::Samurai => "Trace honor nodes while sentries punish mistakes.",
        QuestKind::Mars => "Scan hidden hazards, save battery, and fight dust pushes.",
        QuestKind::DeepSea => "Watch pressure, sonar pings, and oxygen vents.",
        QuestKind::Volcano => "Grab relics while lava rises.",
        QuestKind::Jungle => "Collect relics and avoid traps.",
        QuestKind::Dragon => "Steal gold through hot lair marks with limited lives.",
        QuestKind::Mirror => "Mirrored controls and slide movement.",
    }
}

fn quest_open_point(
    state: &mut AppState,
    w: i32,
    h: i32,
    start: (i32, i32),
    walls: &HashSet<(i32, i32)>,
    blocked: &HashSet<(i32, i32)>,
) -> (i32, i32) {
    for _ in 0..500 {
        let point = (state.rng.range(2, w - 3), state.rng.range(2, h - 3));
        if !walls.contains(&point)
            && !blocked.contains(&point)
            && (walls.is_empty() || path_exists(w, h, start, point, walls))
        {
            return point;
        }
    }
    start
}

fn quest_can_enter(point: (i32, i32), w: i32, h: i32, walls: &HashSet<(i32, i32)>) -> bool {
    point.0 > 0 && point.0 < w - 1 && point.1 > 0 && point.1 < h - 1 && !walls.contains(&point)
}

fn draw_micro_quest(
    state: &AppState,
    name: &str,
    kind: QuestKind,
    w: i32,
    h: i32,
    player: (i32, i32),
    goal: (i32, i32),
    walls: &HashSet<(i32, i32)>,
    items: &HashSet<(i32, i32)>,
    hazards: &HashSet<(i32, i32)>,
    nodes: &[(i32, i32, i32)],
    lava_y: i32,
    steps: u32,
    status: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - h as usize / 2 + 1;
    let left = cols / 2 - w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Steps {steps}   {status}   WASD move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        h as usize + 2,
        w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in walls {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "#",
            &theme,
            Role::Muted,
            false,
        );
    }
    for &(x, y) in items {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "$",
            &theme,
            Role::Success,
            true,
        );
    }
    if !matches!(
        kind,
        QuestKind::Quantum | QuestKind::Mars | QuestKind::DeepSea
    ) {
        for &(x, y) in hazards {
            put(
                &mut buf,
                top + y as usize,
                left + x as usize,
                "^",
                &theme,
                Role::Danger,
                true,
            );
        }
    }
    for &(x, y, n) in nodes {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            &n.to_string(),
            &theme,
            Role::Success,
            true,
        );
    }
    if matches!(kind, QuestKind::Volcano) {
        for x in 1..w - 1 {
            put(
                &mut buf,
                top + lava_y as usize,
                left + x as usize,
                "~",
                &theme,
                Role::Danger,
                true,
            );
        }
    }
    let goal_sprite = if matches!(kind, QuestKind::Checkmate) {
        "K"
    } else {
        "E"
    };
    put(
        &mut buf,
        top + goal.1 as usize,
        left + goal.0 as usize,
        goal_sprite,
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize,
        "@",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

#[derive(Clone, Copy, PartialEq, Eq, Hash)]
enum LaneMeter {
    None,
    Balance,
    Reserve,
    Heat,
    Oxygen,
    Time,
}

#[derive(Clone, Copy)]
struct LaneRules {
    mechanic: &'static str,
    lanes: usize,
    target: u32,
    value: u32,
    spawn_ms: u64,
    speed: f64,
    good_chance: (u32, u32),
    jump_frames: i32,
    meter: LaneMeter,
    meter_drain: i32,
    meter_gain: i32,
    side_push_period: u32,
    miss_good_costs_life: bool,
    extra_bad_chance: (u32, u32),
    player: &'static str,
    good: &'static str,
    bad: &'static str,
    help: &'static str,
}

fn lane_rules(kind: LaneKind) -> LaneRules {
    match kind {
        LaneKind::Rune => LaneRules {
            mechanic: "jump-rune-gates",
            lanes: 4,
            target: 12,
            value: 24,
            spawn_ms: 430,
            speed: 0.47,
            good_chance: (2, 5),
            jump_frames: 3,
            meter: LaneMeter::None,
            meter_drain: 0,
            meter_gain: 0,
            side_push_period: 0,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 5),
            player: "<R>",
            good: "+",
            bad: "|",
            help: "A/D lanes, Space vaults rune gates",
        },
        LaneKind::Sea => LaneRules {
            mechanic: "naval-flag-current",
            lanes: 5,
            target: 11,
            value: 22,
            spawn_ms: 470,
            speed: 0.42,
            good_chance: (1, 3),
            jump_frames: 0,
            meter: LaneMeter::Reserve,
            meter_drain: 1,
            meter_gain: 15,
            side_push_period: 11,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 6),
            player: "<S>",
            good: "F",
            bad: "*",
            help: "Current nudges you; flags refill reserve",
        },
        LaneKind::AirHockey => LaneRules {
            mechanic: "paddle-power-pucks",
            lanes: 3,
            target: 16,
            value: 18,
            spawn_ms: 310,
            speed: 0.66,
            good_chance: (3, 5),
            jump_frames: 0,
            meter: LaneMeter::None,
            meter_drain: 0,
            meter_gain: 0,
            side_push_period: 0,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 3),
            player: "[H]",
            good: "o",
            bad: "O",
            help: "Tiny rink, fast pucks, dodge heavy rebounds",
        },
        LaneKind::Hockey => LaneRules {
            mechanic: "checking-breakaway",
            lanes: 5,
            target: 13,
            value: 25,
            spawn_ms: 390,
            speed: 0.55,
            good_chance: (2, 5),
            jump_frames: 0,
            meter: LaneMeter::Heat,
            meter_drain: -1,
            meter_gain: -18,
            side_push_period: 9,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 4),
            player: "/H\\",
            good: "o",
            bad: "#",
            help: "Checks shove lanes; pucks cool the breakaway",
        },
        LaneKind::Ski => LaneRules {
            mechanic: "mandatory-slalom-gates",
            lanes: 5,
            target: 15,
            value: 28,
            spawn_ms: 360,
            speed: 0.58,
            good_chance: (1, 1),
            jump_frames: 0,
            meter: LaneMeter::None,
            meter_drain: 0,
            meter_gain: 0,
            side_push_period: 0,
            miss_good_costs_life: true,
            extra_bad_chance: (0, 1),
            player: "/S\\",
            good: "G",
            bad: "^",
            help: "Hit every slalom gate; missed gates cost lives",
        },
        LaneKind::Snowboard => LaneRules {
            mechanic: "balance-rail-sparks",
            lanes: 4,
            target: 14,
            value: 27,
            spawn_ms: 410,
            speed: 0.48,
            good_chance: (1, 2),
            jump_frames: 0,
            meter: LaneMeter::Balance,
            meter_drain: 2,
            meter_gain: 5,
            side_push_period: 0,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 5),
            player: "/R\\",
            good: "*",
            bad: "X",
            help: "W/S trims balance while you collect sparks",
        },
        LaneKind::Bmx => LaneRules {
            mechanic: "bunny-hop-repair-run",
            lanes: 6,
            target: 13,
            value: 24,
            spawn_ms: 350,
            speed: 0.57,
            good_chance: (2, 5),
            jump_frames: 2,
            meter: LaneMeter::None,
            meter_drain: 0,
            meter_gain: 0,
            side_push_period: 0,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 3),
            player: "/X\\",
            good: "+",
            bad: "[]",
            help: "Six-lane alley; Space bunny-hops debris",
        },
        LaneKind::Horse => LaneRules {
            mechanic: "stamina-hurdle-dash",
            lanes: 4,
            target: 14,
            value: 26,
            spawn_ms: 420,
            speed: 0.52,
            good_chance: (2, 5),
            jump_frames: 3,
            meter: LaneMeter::Reserve,
            meter_drain: 2,
            meter_gain: 20,
            side_push_period: 0,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 4),
            player: "/H\\",
            good: "U",
            bad: "|",
            help: "Space jumps hurdles; horseshoes refill stamina",
        },
        LaneKind::Ninja => LaneRules {
            mechanic: "rooftop-scroll-jumps",
            lanes: 5,
            target: 12,
            value: 30,
            spawn_ms: 330,
            speed: 0.62,
            good_chance: (1, 3),
            jump_frames: 2,
            meter: LaneMeter::None,
            meter_drain: 0,
            meter_gain: 0,
            side_push_period: 7,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 3),
            player: "<N>",
            good: "S",
            bad: "#",
            help: "Wind shifts rooftops; jump gaps with Space",
        },
        LaneKind::Moon => LaneRules {
            mechanic: "low-gravity-ore-hop",
            lanes: 5,
            target: 10,
            value: 32,
            spawn_ms: 500,
            speed: 0.36,
            good_chance: (2, 5),
            jump_frames: 5,
            meter: LaneMeter::Oxygen,
            meter_drain: 1,
            meter_gain: 16,
            side_push_period: 13,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 6),
            player: "/M\\",
            good: "o",
            bad: "#",
            help: "Long moon hops; oxygen ticks down",
        },
        LaneKind::Saturn => LaneRules {
            mechanic: "ring-orbit-lanes",
            lanes: 6,
            target: 15,
            value: 22,
            spawn_ms: 340,
            speed: 0.61,
            good_chance: (1, 3),
            jump_frames: 0,
            meter: LaneMeter::None,
            meter_drain: 0,
            meter_gain: 0,
            side_push_period: 5,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 4),
            player: "[S]",
            good: "o",
            bad: "*",
            help: "Ring gravity shoves lanes every few beats",
        },
        LaneKind::Submarine => LaneRules {
            mechanic: "oxygen-mine-sweep",
            lanes: 4,
            target: 12,
            value: 25,
            spawn_ms: 440,
            speed: 0.45,
            good_chance: (1, 3),
            jump_frames: 0,
            meter: LaneMeter::Oxygen,
            meter_drain: 2,
            meter_gain: 24,
            side_push_period: 0,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 4),
            player: "<U>",
            good: "O",
            bad: "*",
            help: "Oxygen drops fast; tanks are mandatory",
        },
        LaneKind::Desert => LaneRules {
            mechanic: "heat-water-caravan",
            lanes: 5,
            target: 12,
            value: 24,
            spawn_ms: 460,
            speed: 0.44,
            good_chance: (1, 3),
            jump_frames: 0,
            meter: LaneMeter::Reserve,
            meter_drain: 2,
            meter_gain: 22,
            side_push_period: 8,
            miss_good_costs_life: false,
            extra_bad_chance: (1, 5),
            player: "/C\\",
            good: "W",
            bad: "~",
            help: "Mirages shove you; water keeps the caravan alive",
        },
        LaneKind::Time => LaneRules {
            mechanic: "time-spark-speedrun",
            lanes: 5,
            target: 16,
            value: 21,
            spawn_ms: 280,
            speed: 0.72,
            good_chance: (2, 5),
            jump_frames: 0,
            meter: LaneMeter::Time,
            meter_drain: 3,
            meter_gain: 13,
            side_push_period: 4,
            miss_good_costs_life: true,
            extra_bad_chance: (1, 3),
            player: "<T>",
            good: "*",
            bad: "|",
            help: "Fastest lanes; sparks buy time, misses hurt",
        },
    }
}

fn game_micro_lane(state: &mut AppState, name: &str, kind: LaneKind) {
    if !require_size(state, 22, 58, name) {
        return;
    }
    loop {
        let rules = lane_rules(kind);
        let lanes = rules.lanes;
        let track_h = 14i32;
        let mut player_lane = 2usize;
        let mut objects: Vec<(usize, f64, bool)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut collected = 0u32;
        let target = rules.target + state.difficulty_index as u32;
        let mut meter = 80i32;
        let mut jump = 0i32;
        let mut tick = 0u32;
        let mut last_spawn = Instant::now();
        while lives > 0 && collected < target {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') if player_lane > 0 => player_lane -= 1,
                    Key::Right | Key::Char('d') if player_lane + 1 < lanes => player_lane += 1,
                    Key::Up | Key::Char('w') if rules.meter == LaneMeter::Balance => {
                        meter += rules.meter_gain.max(1);
                    }
                    Key::Down | Key::Char('s') if rules.meter == LaneMeter::Balance => {
                        meter -= rules.meter_gain.max(1);
                    }
                    Key::Enter | Key::Space if rules.jump_frames > 0 => {
                        jump = rules.jump_frames;
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            tick += 1;
            if jump > 0 {
                jump -= 1;
            }
            if rules.side_push_period > 0 && tick % rules.side_push_period == 0 {
                let push_right = (tick / rules.side_push_period) % 2 == 0;
                if push_right && player_lane + 1 < lanes {
                    player_lane += 1;
                } else if !push_right && player_lane > 0 {
                    player_lane -= 1;
                }
            }
            if rules.meter != LaneMeter::None && tick % 8 == 0 {
                meter -= rules.meter_drain + state.difficulty_index as i32;
                if rules.meter == LaneMeter::Heat {
                    meter += 5 + state.difficulty_index as i32;
                }
                if meter <= 0 || meter >= 125 {
                    lives -= 1;
                    meter = 70;
                    play_sound(state, "alert");
                }
            }
            if rules.meter == LaneMeter::Balance {
                meter += if tick % 2 == 0 { -rules.meter_drain } else { 1 };
                if !(20..=120).contains(&meter) {
                    lives -= 1;
                    meter = 80;
                    play_sound(state, "alert");
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((rules.spawn_ms as f64 / state.difficulty().speed) as u64)
            {
                let good = state.rng.chance(rules.good_chance.0, rules.good_chance.1);
                objects.push((state.rng.usize(lanes), 1.0, good));
                if state.difficulty_index > 0
                    && state
                        .rng
                        .chance(rules.extra_bad_chance.0, rules.extra_bad_chance.1)
                {
                    objects.push((state.rng.usize(lanes), 1.0, false));
                }
                last_spawn = Instant::now();
            }
            for object in &mut objects {
                object.1 += rules.speed * state.difficulty().speed;
            }
            let mut kept = Vec::new();
            for (lane, y, good) in objects.into_iter() {
                let at_player = y.round() as i32 >= track_h - 2 && lane == player_lane;
                if y.round() as i32 >= track_h {
                    if rules.miss_good_costs_life && good && lane != player_lane {
                        lives -= 1;
                        play_sound(state, "alert");
                    }
                    continue;
                }
                if at_player {
                    if good {
                        collected += 1;
                        score += rules.value;
                        if rules.meter != LaneMeter::None {
                            if rules.meter == LaneMeter::Heat {
                                meter = (meter + rules.meter_gain).clamp(0, 120);
                            } else {
                                meter = (meter + rules.meter_gain).min(120);
                            }
                        }
                        play_sound(state, "score");
                    } else if jump > 0 {
                        score += 8 + rules.value / 5;
                    } else {
                        lives -= 1;
                        if rules.meter == LaneMeter::Heat {
                            meter = (meter + 12).min(120);
                        }
                        play_sound(state, "alert");
                    }
                } else {
                    kept.push((lane, y, good));
                }
            }
            objects = kept;
            draw_micro_lane(
                state,
                name,
                kind,
                lanes,
                track_h,
                player_lane,
                &objects,
                lives,
                score,
                collected,
                target,
                meter,
                jump,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        let final_score = score + if collected >= target { 350 } else { 0 };
        record_score(state, name, final_score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Run score: {final_score}"),
                format!("Collected {collected}/{target}."),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_micro_lane(
    state: &AppState,
    name: &str,
    kind: LaneKind,
    lanes: usize,
    track_h: i32,
    player_lane: usize,
    objects: &[(usize, f64, bool)],
    lives: i32,
    score: u32,
    collected: u32,
    target: u32,
    meter: i32,
    jump: i32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - track_h as usize / 2 + 1;
    let left = cols / 2 - 16;
    let lane_gap = 8usize;
    let rules = lane_rules(kind);
    let status = match rules.meter {
        LaneMeter::Balance => format!("Balance {meter}"),
        LaneMeter::Reserve => format!("Reserve {meter}"),
        LaneMeter::Heat => format!("Heat {meter}"),
        LaneMeter::Oxygen => format!("O2 {meter}"),
        LaneMeter::Time => format!("Time {meter}"),
        LaneMeter::None if jump > 0 => "Jump".to_string(),
        LaneMeter::None => rules.mechanic.to_string(),
    };
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   Goal {collected}/{target}   {status}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for lane in 0..lanes {
        let x = left + lane * lane_gap;
        for y in 0..track_h {
            put(
                &mut buf,
                top + y as usize,
                x,
                "|",
                &theme,
                Role::Muted,
                false,
            );
        }
    }
    for &(lane, y, is_good) in objects {
        let x = left + lane * lane_gap;
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            x.saturating_sub(1),
            if is_good { rules.good } else { rules.bad },
            &theme,
            if is_good { Role::Success } else { Role::Danger },
            true,
        );
    }
    put(
        &mut buf,
        top + track_h as usize - 1,
        left + player_lane * lane_gap - 1,
        rules.player,
        &theme,
        if jump > 0 {
            Role::Highlight
        } else {
            Role::Secondary
        },
        true,
    );
    center(
        &mut buf,
        top + track_h as usize + 2,
        rules.help,
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

#[derive(Clone, Copy)]
struct CatchRules {
    mechanic: &'static str,
    target: u32,
    speed: f64,
    spawn_ms: u64,
    good_chance: (u32, u32),
    catch_width: i32,
    player_step: i32,
    needs_action: bool,
    miss_good_costs_life: bool,
    combo: bool,
    recipe: bool,
    shield: bool,
    player: &'static str,
    good: &'static str,
    bad: &'static str,
    help: &'static str,
}

fn catch_rules(kind: CatchKind) -> CatchRules {
    match kind {
        CatchKind::Glyph => CatchRules {
            mechanic: "rare-glyph-combo",
            target: 14,
            speed: 0.40,
            spawn_ms: 430,
            good_chance: (1, 3),
            catch_width: 2,
            player_step: 2,
            needs_action: false,
            miss_good_costs_life: true,
            combo: true,
            recipe: false,
            shield: false,
            player: "{V}",
            good: "*",
            bad: "x",
            help: "Catch glyph blooms; misses break the combo",
        },
        CatchKind::Pinball => CatchRules {
            mechanic: "flipper-timing-bumpers",
            target: 20,
            speed: 0.66,
            spawn_ms: 300,
            good_chance: (3, 4),
            catch_width: 3,
            player_step: 3,
            needs_action: true,
            miss_good_costs_life: false,
            combo: true,
            recipe: false,
            shield: false,
            player: "[=]",
            good: "o",
            bad: "v",
            help: "Space flips when the ball reaches the flipper",
        },
        CatchKind::Tennis => CatchRules {
            mechanic: "rally-sweet-spot",
            target: 12,
            speed: 0.56,
            spawn_ms: 390,
            good_chance: (2, 3),
            catch_width: 1,
            player_step: 2,
            needs_action: true,
            miss_good_costs_life: true,
            combo: true,
            recipe: false,
            shield: false,
            player: "[T]",
            good: "o",
            bad: "F",
            help: "Line up and press Space in the sweet spot",
        },
        CatchKind::Cricket => CatchRules {
            mechanic: "wicket-catch-bouncers",
            target: 11,
            speed: 0.52,
            spawn_ms: 420,
            good_chance: (1, 2),
            catch_width: 2,
            player_step: 2,
            needs_action: true,
            miss_good_costs_life: true,
            combo: false,
            recipe: false,
            shield: false,
            player: "[C]",
            good: "o",
            bad: "b",
            help: "Space catches balls; bouncers punish bad reads",
        },
        CatchKind::Alien => CatchRules {
            mechanic: "alien-orchard-harvest",
            target: 13,
            speed: 0.44,
            spawn_ms: 410,
            good_chance: (2, 5),
            catch_width: 2,
            player_step: 3,
            needs_action: false,
            miss_good_costs_life: false,
            combo: true,
            recipe: false,
            shield: false,
            player: "{O}",
            good: "@",
            bad: "x",
            help: "Wide alien basket; harvest streaks score big",
        },
        CatchKind::Astro => CatchRules {
            mechanic: "orbit-crop-bad-seeds",
            target: 15,
            speed: 0.36,
            spawn_ms: 360,
            good_chance: (1, 2),
            catch_width: 1,
            player_step: 1,
            needs_action: false,
            miss_good_costs_life: true,
            combo: false,
            recipe: false,
            shield: false,
            player: "[F]",
            good: "%",
            bad: "b",
            help: "Slow precise farming; bad seeds are costly",
        },
        CatchKind::Castle => CatchRules {
            mechanic: "siege-supply-shield",
            target: 16,
            speed: 0.48,
            spawn_ms: 350,
            good_chance: (2, 5),
            catch_width: 3,
            player_step: 2,
            needs_action: false,
            miss_good_costs_life: false,
            combo: false,
            recipe: false,
            shield: true,
            player: "[C]",
            good: "+",
            bad: "O",
            help: "Supplies repair shields; stones crack them",
        },
        CatchKind::Potion => CatchRules {
            mechanic: "three-step-potion-recipe",
            target: 12,
            speed: 0.46,
            spawn_ms: 460,
            good_chance: (2, 3),
            catch_width: 2,
            player_step: 2,
            needs_action: false,
            miss_good_costs_life: true,
            combo: false,
            recipe: true,
            shield: false,
            player: "[P]",
            good: "!",
            bad: "~",
            help: "Keep the recipe chain alive; smoke resets it",
        },
        CatchKind::Poker => CatchRules {
            mechanic: "draw-poker-hand",
            target: 5,
            speed: 0.0,
            spawn_ms: 0,
            good_chance: (0, 1),
            catch_width: 0,
            player_step: 0,
            needs_action: false,
            miss_good_costs_life: false,
            combo: false,
            recipe: false,
            shield: false,
            player: "[_]",
            good: "*",
            bad: "x",
            help: "Draw a five-card hand",
        },
    }
}

fn game_micro_catch(state: &mut AppState, name: &str, kind: CatchKind) {
    if matches!(kind, CatchKind::Poker) {
        return game_poker_draw(state, name);
    }
    if !require_size(state, 22, 62, name) {
        return;
    }
    loop {
        let (w, h) = full_board(48, 16, 112, 34);
        let mut player_x = w / 2;
        let mut objects: Vec<(i32, f64, bool)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut caught = 0u32;
        let mut recipe = 0usize;
        let mut streak = 1u32;
        let mut shield = 3i32;
        let rules = catch_rules(kind);
        let target = rules.target + state.difficulty_index as u32;
        let mut last_spawn = Instant::now();
        while lives > 0 && caught < target {
            let frame = Instant::now();
            let mut action = false;
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => player_x = (player_x - rules.player_step).max(2),
                    Key::Right | Key::Char('d') => {
                        player_x = (player_x + rules.player_step).min(w - 3)
                    }
                    Key::Enter | Key::Space => action = true,
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((rules.spawn_ms as f64 / state.difficulty().speed) as u64)
            {
                let good = state.rng.chance(rules.good_chance.0, rules.good_chance.1);
                objects.push((state.rng.range(2, w - 3), 1.0, good));
                last_spawn = Instant::now();
            }
            for object in &mut objects {
                object.1 += rules.speed * state.difficulty().speed;
            }
            let mut kept = Vec::new();
            for (x, y, good) in objects.into_iter() {
                let oy = y.round() as i32;
                let aligned = oy >= h - 2 && (x - player_x).abs() <= rules.catch_width;
                if oy >= h {
                    if good && rules.miss_good_costs_life {
                        lives -= 1;
                        streak = 1;
                        play_sound(state, "alert");
                    }
                    continue;
                }
                if aligned && (!rules.needs_action || action) {
                    if good {
                        caught += 1;
                        let recipe_bonus = if rules.recipe {
                            recipe = (recipe + 1) % 3;
                            10 + recipe as u32 * 5
                        } else {
                            0
                        };
                        let streak_bonus = if rules.combo {
                            let bonus = streak.min(8);
                            streak = (streak + 1).min(9);
                            bonus * 3
                        } else {
                            0
                        };
                        if rules.shield {
                            shield = (shield + 1).min(5);
                        }
                        score += catch_value(kind) + recipe_bonus + streak_bonus;
                        play_sound(state, "score");
                    } else {
                        if rules.shield && shield > 0 {
                            shield -= 1;
                        } else {
                            lives -= 1;
                        }
                        recipe = 0;
                        streak = 1;
                        play_sound(state, "alert");
                    }
                } else {
                    kept.push((x, y, good));
                }
            }
            objects = kept;
            draw_micro_catch(
                state, name, kind, w, h, player_x, &objects, lives, score, caught, target, recipe,
                streak, shield,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        let final_score = score + if caught >= target { 250 } else { 0 };
        record_score(state, name, final_score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Final score: {final_score}"),
                format!("Caught {caught}/{target}."),
            ],
            true,
        ) {
            return;
        }
    }
}

fn catch_value(kind: CatchKind) -> u32 {
    match kind {
        CatchKind::Pinball => 12,
        CatchKind::Tennis | CatchKind::Cricket => 24,
        CatchKind::Potion => 18,
        _ => 16,
    }
}

fn draw_micro_catch(
    state: &AppState,
    name: &str,
    kind: CatchKind,
    w: i32,
    h: i32,
    player_x: i32,
    objects: &[(i32, f64, bool)],
    lives: i32,
    score: u32,
    caught: u32,
    target: u32,
    recipe: usize,
    streak: u32,
    shield: i32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - h as usize / 2 + 1;
    let left = cols / 2 - w as usize / 2;
    let rules = catch_rules(kind);
    let extra = if rules.recipe {
        format!("   Recipe step {}", recipe + 1)
    } else if rules.needs_action {
        "   Space times the hit".to_string()
    } else if rules.combo {
        format!("   Streak x{streak}")
    } else if rules.shield {
        format!("   Shield {shield}")
    } else {
        format!("   {}", rules.mechanic)
    };
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   Goal {caught}/{target}{extra}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        h as usize + 2,
        w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y, is_good) in objects {
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + x as usize,
            if is_good { rules.good } else { rules.bad },
            &theme,
            if is_good { Role::Success } else { Role::Danger },
            true,
        );
    }
    put(
        &mut buf,
        top + h as usize - 2,
        left + player_x as usize - rules.player.len() / 2,
        rules.player,
        &theme,
        Role::Secondary,
        true,
    );
    center(
        &mut buf,
        top + h as usize + 1,
        rules.help,
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_poker_draw(state: &mut AppState, name: &str) {
    loop {
        let mut hand = Vec::new();
        let mut draws = 0u32;
        while hand.len() < 5 {
            draw_poker_hand(state, name, &hand, draws, "Space draws a card.");
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Enter | Key::Space => {
                        hand.push(state.rng.range(1, 13) as u8);
                        draws += 1;
                        play_sound(state, "score");
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let mut counts = [0u8; 14];
        for &card in &hand {
            counts[card as usize] += 1;
        }
        let pairs = counts.iter().filter(|&&n| n == 2).count() as u32;
        let triples = counts.iter().filter(|&&n| n == 3).count() as u32;
        let quads = counts.iter().filter(|&&n| n == 4).count() as u32;
        let score = quads * 700
            + triples * 320
            + pairs * 150
            + hand.iter().map(|&c| c as u32).sum::<u32>() * 4;
        draw_poker_hand(state, name, &hand, draws, "Hand scored.");
        record_score(state, name, score);
        if !wait_menu(state, name, &[format!("Poker score: {score}")], true) {
            return;
        }
    }
}

fn draw_poker_hand(state: &AppState, name: &str, hand: &[u8], draws: u32, message: &str) {
    let (_, cols) = terminal_size();
    let theme = state.theme().clone();
    let cards = hand
        .iter()
        .map(|&card| blackjack_card_label(card).to_string())
        .collect::<Vec<_>>()
        .join(" ");
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        5,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        9,
        &format!("Draw {draws}/5"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    center(
        &mut buf,
        12,
        &format!("[ {cards} ]"),
        &theme,
        Role::Success,
        true,
        cols,
    );
    center(&mut buf, 16, message, &theme, Role::Secondary, true, cols);
    flush(&buf);
}

#[derive(Clone, Copy)]
struct AimRules {
    mechanic: &'static str,
    target: &'static str,
    shots: u32,
    aim_limit: i32,
    power_min: i32,
    power_max: i32,
    ideal_power: i32,
    wind_limit: i32,
    target_drift: i32,
    aim_weight: i32,
    help: &'static str,
}

fn aim_rules(kind: AimKind) -> AimRules {
    match kind {
        AimKind::Basket => AimRules {
            mechanic: "moving-hoop-arc-shot",
            target: "(O)",
            shots: 9,
            aim_limit: 6,
            power_min: 2,
            power_max: 11,
            ideal_power: 7,
            wind_limit: 2,
            target_drift: 3,
            aim_weight: 1,
            help: "A/D aim, W/S arc power; moving hoop changes lanes",
        },
        AimKind::Archery => AimRules {
            mechanic: "bullseye-wind-compensation",
            target: "<O>",
            shots: 7,
            aim_limit: 8,
            power_min: 3,
            power_max: 12,
            ideal_power: 6,
            wind_limit: 4,
            target_drift: 1,
            aim_weight: 2,
            help: "Wind matters twice as much; power controls arrow drop",
        },
        AimKind::Curling => AimRules {
            mechanic: "curling-weight-ice-read",
            target: "((O))",
            shots: 8,
            aim_limit: 5,
            power_min: 1,
            power_max: 10,
            ideal_power: 5,
            wind_limit: 1,
            target_drift: 4,
            aim_weight: 1,
            help: "A/D line, W/S weight; ice curl moves the house",
        },
    }
}

fn game_micro_aim(state: &mut AppState, name: &str, kind: AimKind) {
    if !require_size(state, 20, 58, name) {
        return;
    }
    loop {
        let rules = aim_rules(kind);
        let mut aim = 0i32;
        let mut power = rules.ideal_power;
        let mut shots = 0u32;
        let mut score = 0u32;
        let mut wind = state.rng.range(-rules.wind_limit, rules.wind_limit);
        let mut target_offset = state.rng.range(-rules.target_drift, rules.target_drift);
        let mut ice_read = 0i32;
        while shots < rules.shots {
            draw_micro_aim(
                state,
                name,
                kind,
                aim,
                power,
                wind,
                target_offset,
                ice_read,
                shots,
                rules.shots,
                score,
            );
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Left | Key::Char('a') => aim = (aim - 1).max(-rules.aim_limit),
                    Key::Right | Key::Char('d') => aim = (aim + 1).min(rules.aim_limit),
                    Key::Up | Key::Char('w') => power = (power + 1).min(rules.power_max),
                    Key::Down | Key::Char('s') => power = (power - 1).max(rules.power_min),
                    Key::Enter | Key::Space => {
                        let terrain = match kind {
                            AimKind::Basket => target_offset,
                            AimKind::Archery => wind,
                            AimKind::Curling => {
                                ice_read = state.rng.range(-2, 2);
                                target_offset + ice_read
                            }
                        };
                        let error = ((aim + wind - terrain).abs() * rules.aim_weight)
                            + (power - rules.ideal_power).abs();
                        let points = match error {
                            0 => 100,
                            1 => 60,
                            2 => 35,
                            3 => 15,
                            _ => 0,
                        };
                        score += points;
                        shots += 1;
                        wind = state.rng.range(-rules.wind_limit, rules.wind_limit);
                        target_offset = state.rng.range(-rules.target_drift, rules.target_drift);
                        if points > 0 {
                            play_sound(state, "score");
                        } else {
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        record_score(state, name, score);
        if !wait_menu(state, name, &[format!("Final score: {score}")], true) {
            return;
        }
    }
}

fn draw_micro_aim(
    state: &AppState,
    name: &str,
    kind: AimKind,
    aim: i32,
    power: i32,
    wind: i32,
    target_offset: i32,
    ice_read: i32,
    shots: u32,
    max_shots: u32,
    score: u32,
) {
    let (_, cols) = terminal_size();
    let theme = state.theme().clone();
    let rules = aim_rules(kind);
    let marker_left = (aim + rules.aim_limit) as usize;
    let marker_right = (rules.aim_limit - aim) as usize;
    let target_pad = (target_offset + rules.target_drift).max(0) as usize;
    let terrain = match kind {
        AimKind::Basket => format!("Hoop lane {target_offset:+}   {}", rules.mechanic),
        AimKind::Archery => format!("Wind compensation {wind:+}   {}", rules.mechanic),
        AimKind::Curling => {
            format!(
                "House {target_offset:+}   Ice curl {ice_read:+}   {}",
                rules.mechanic
            )
        }
    };
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        3,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        6,
        &format!("Score {score}   Shot {shots}/{max_shots}   Wind {wind}   Power {power}"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    center(&mut buf, 8, &terrain, &theme, Role::Secondary, true, cols);
    center(
        &mut buf,
        10,
        &format!("{}{}", " ".repeat(target_pad), rules.target),
        &theme,
        Role::Success,
        true,
        cols,
    );
    center(
        &mut buf,
        13,
        &format!(
            "Aim: {}^{}",
            " ".repeat(marker_left),
            " ".repeat(marker_right)
        ),
        &theme,
        Role::Highlight,
        true,
        cols,
    );
    center(&mut buf, 17, rules.help, &theme, Role::Muted, false, cols);
    flush(&buf);
}

const FACTORY_KEYS: [char; 4] = ['w', 'a', 's', 'd'];
const DUEL_KEYS: [char; 4] = ['w', 'a', 's', 'd'];
const TRICK_KEYS: [char; 4] = ['a', 'd', 'w', 's'];

#[derive(Clone, Copy)]
struct SequenceRules {
    mechanic: &'static str,
    label: &'static str,
    keys: &'static [char],
    base_len: usize,
    mistakes_allowed: u32,
    reset_on_miss: bool,
    step_back_on_miss: bool,
    bonus_per_step: u32,
    help: &'static str,
}

fn sequence_rules(kind: SequenceKind, difficulty_index: usize) -> SequenceRules {
    let extra = difficulty_index * 2;
    match kind {
        SequenceKind::Factory => SequenceRules {
            mechanic: "assembly-queue-no-reset",
            label: "Assemble the recipe",
            keys: &FACTORY_KEYS,
            base_len: 5 + extra,
            mistakes_allowed: 4,
            reset_on_miss: false,
            step_back_on_miss: false,
            bonus_per_step: 45,
            help: "Wrong inputs waste parts, but the belt keeps moving",
        },
        SequenceKind::Duel => SequenceRules {
            mechanic: "spell-chain-hard-reset",
            label: "Counter the spell chain",
            keys: &DUEL_KEYS,
            base_len: 4 + extra,
            mistakes_allowed: 2,
            reset_on_miss: true,
            step_back_on_miss: false,
            bonus_per_step: 70,
            help: "A miss breaks the ward and restarts the spell",
        },
        SequenceKind::Trick => SequenceRules {
            mechanic: "skate-combo-stepback",
            label: "Land the trick combo",
            keys: &TRICK_KEYS,
            base_len: 6 + extra,
            mistakes_allowed: 3,
            reset_on_miss: false,
            step_back_on_miss: true,
            bonus_per_step: 55,
            help: "A wobble drops one trick from the combo",
        },
    }
}

fn game_micro_sequence(state: &mut AppState, name: &str, kind: SequenceKind) {
    loop {
        let rules = sequence_rules(kind, state.difficulty_index);
        let target_len = rules.base_len;
        let mut sequence = Vec::new();
        for _ in 0..target_len {
            sequence.push(rules.keys[state.rng.usize(rules.keys.len())]);
        }
        let mut index = 0usize;
        let mut mistakes = 0u32;
        let mut chain = 0u32;
        while index < sequence.len() && mistakes < rules.mistakes_allowed {
            draw_micro_sequence(state, name, kind, &sequence, index, mistakes);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Char(ch) if ['w', 'a', 's', 'd'].contains(&ch) => {
                        if ch == sequence[index] {
                            index += 1;
                            chain += 1;
                            play_sound(state, "score");
                        } else {
                            mistakes += 1;
                            chain = 0;
                            if rules.reset_on_miss {
                                index = 0;
                            } else if rules.step_back_on_miss && index > 0 {
                                index -= 1;
                            }
                            play_sound(state, "alert");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let score = if index == sequence.len() {
            500u32
                .saturating_add(chain * rules.bonus_per_step)
                .saturating_sub(mistakes * 80)
        } else {
            index as u32 * rules.bonus_per_step
        };
        record_score(state, name, score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Sequence score: {score}"),
                format!("Mistakes: {mistakes}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_micro_sequence(
    state: &AppState,
    name: &str,
    kind: SequenceKind,
    sequence: &[char],
    index: usize,
    mistakes: u32,
) {
    let (_, cols) = terminal_size();
    let theme = state.theme().clone();
    let rules = sequence_rules(kind, 1);
    let label = rules.label;
    let shown = sequence
        .iter()
        .enumerate()
        .map(|(i, ch)| {
            if i < index {
                format!("({ch})")
            } else {
                format!(" {ch} ")
            }
        })
        .collect::<Vec<_>>()
        .join(" ");
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        4,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        7,
        &format!("{label}   {}", rules.mechanic),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    center(&mut buf, 11, &shown, &theme, Role::Success, true, cols);
    center(
        &mut buf,
        15,
        &format!(
            "Step {}/{}   Mistakes {mistakes}/{}",
            index + 1,
            sequence.len(),
            rules.mistakes_allowed
        ),
        &theme,
        Role::Secondary,
        true,
        cols,
    );
    center(&mut buf, 17, rules.help, &theme, Role::Muted, false, cols);
    flush(&buf);
}

fn game_snake(state: &mut AppState) {
    if !require_size(state, 22, 58, "Snake") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(42, 16, 118, 40);
        let mut snake = VecDeque::new();
        snake.push_front((board_w / 2, board_h / 2));
        snake.push_back((board_w / 2 - 1, board_h / 2));
        snake.push_back((board_w / 2 - 2, board_h / 2));
        let mut dir = (1, 0);
        let mut food = random_empty(state, board_w, board_h, &snake.iter().copied().collect());
        let mut score = 0u32;
        let tick = (state.difficulty().tick_ms as f64 * 1.25) as u64;
        let mut alive = true;
        while alive {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') if dir != (0, 1) => dir = (0, -1),
                    Key::Down | Key::Char('s') if dir != (0, -1) => dir = (0, 1),
                    Key::Left | Key::Char('a') if dir != (1, 0) => dir = (-1, 0),
                    Key::Right | Key::Char('d') if dir != (-1, 0) => dir = (1, 0),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            let (hx, hy) = snake[0];
            let new_head = (hx + dir.0, hy + dir.1);
            if new_head.0 < 0
                || new_head.1 < 0
                || new_head.0 >= board_w
                || new_head.1 >= board_h
                || snake.contains(&new_head)
            {
                alive = false;
            } else {
                snake.push_front(new_head);
                if new_head == food {
                    score += 10;
                    play_sound(state, "score");
                    let occupied: HashSet<_> = snake.iter().copied().collect();
                    food = random_empty(state, board_w, board_h, &occupied);
                } else {
                    snake.pop_back();
                }
            }
            draw_snake(state, board_w, board_h, &snake, food, score);
            sleep_frame(frame, tick);
        }
        record_score(state, "Snake", score);
        if !wait_menu(
            state,
            "Snake",
            &[
                format!("Game over. Score: {score}"),
                "You ran out of room.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn random_empty(
    state: &mut AppState,
    w: i32,
    h: i32,
    occupied: &HashSet<(i32, i32)>,
) -> (i32, i32) {
    loop {
        let p = (state.rng.range(0, w - 1), state.rng.range(0, h - 1));
        if !occupied.contains(&p) {
            return p;
        }
    }
}

fn draw_snake(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    snake: &VecDeque<(i32, i32)>,
    food: (i32, i32),
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "SNAKE", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!(
            "Score {score}   {}   WASD/arrows move   Q menu",
            state.difficulty().name
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    put(
        &mut buf,
        top + food.1 as usize,
        left + food.0 as usize,
        "*",
        &theme,
        Role::Success,
        true,
    );
    for (i, (x, y)) in snake.iter().enumerate() {
        put(
            &mut buf,
            top + *y as usize,
            left + *x as usize,
            if i == 0 { "@" } else { "o" },
            &theme,
            Role::Secondary,
            true,
        );
    }
    flush(&buf);
}

fn game_pong(state: &mut AppState) {
    if !require_size(state, 22, 70, "Pong") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(60, 17, 132, 38);
        let (player_paddle_h, ai_paddle_h) = match state.difficulty_index {
            0 => (9, 4),
            1 => (8, 5),
            2 => (7, 5),
            _ => (6, 6),
        };
        let mut player_y = board_h / 2 - player_paddle_h / 2;
        let mut ai_y = board_h / 2 - ai_paddle_h / 2;
        let mut ball_x = board_w as f64 / 2.0;
        let mut ball_y = board_h as f64 / 2.0;
        let speed_factor = state.pong_speed_factor();
        let mut vel_x = match state.difficulty_index {
            0 => 0.46,
            1 => 0.58,
            2 => 0.70,
            _ => 0.82,
        } * speed_factor;
        let mut vel_y = match state.difficulty_index {
            0 => 0.22,
            1 => 0.28,
            2 => 0.34,
            _ => 0.40,
        } * speed_factor;
        let max_ball_speed = match state.difficulty_index {
            0 => 0.92,
            1 => 1.04,
            2 => 1.18,
            _ => 1.32,
        } * speed_factor;
        let player_step = match state.pong_assist_index {
            0 => 2,
            1 => 3,
            _ => 4,
        };
        let player_hit_pad = state.pong_assist_index as i32;
        let ai_reaction = match state.difficulty_index {
            0 => 5,
            1 => 4,
            2 => 3,
            _ => 2,
        };
        let ai_error = match state.difficulty_index {
            0 => 4,
            1 => 3,
            2 => 2,
            _ => 1,
        };
        let ai_step = match state.difficulty_index {
            0 | 1 => 1,
            _ => 2,
        };
        let mut player_score = 0;
        let mut ai_score = 0;
        let win_score = if state.endless_mode { 99 } else { 5 };
        let mut tick = 0u32;
        while player_score < win_score && ai_score < win_score {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => player_y = (player_y - player_step).max(0),
                    Key::Down | Key::Char('s') => {
                        player_y = (player_y + player_step).min(board_h - player_paddle_h)
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            tick += 1;
            if state.pong_assist_index > 0 && vel_x < 0.0 && ball_x < board_w as f64 * 0.58 {
                let player_center = player_y + player_paddle_h / 2;
                let target = ball_y.round() as i32;
                let assist_step = state.pong_assist_index as i32;
                if player_center < target {
                    player_y = (player_y + assist_step).min(board_h - player_paddle_h);
                } else if player_center > target {
                    player_y = (player_y - assist_step).max(0);
                }
            }
            if tick % ai_reaction == 0 {
                let target =
                    ball_y.round() as i32 - ai_paddle_h / 2 + state.rng.range(-ai_error, ai_error);
                if ai_y + ai_paddle_h / 2 < target {
                    ai_y = (ai_y + ai_step).min(board_h - ai_paddle_h);
                } else if ai_y + ai_paddle_h / 2 > target {
                    ai_y = (ai_y - ai_step).max(0);
                }
            }
            ball_x += vel_x;
            ball_y += vel_y;
            if ball_y <= 0.0 || ball_y >= (board_h - 1) as f64 {
                vel_y = -vel_y;
                ball_y = ball_y.clamp(0.0, (board_h - 1) as f64);
                play_sound(state, "wall");
            }
            let by = ball_y.round() as i32;
            if ball_x <= 2.0
                && vel_x < 0.0
                && by >= player_y - player_hit_pad
                && by < player_y + player_paddle_h + player_hit_pad
            {
                vel_x = (vel_x.abs() + 0.025).min(max_ball_speed);
                let hit_y = by.clamp(player_y, player_y + player_paddle_h - 1);
                vel_y += ((hit_y - player_y) as f64 / player_paddle_h as f64 - 0.5) * 0.26;
                play_sound(state, "paddle");
            }
            if ball_x >= (board_w - 3) as f64
                && vel_x > 0.0
                && by >= ai_y
                && by < ai_y + ai_paddle_h
            {
                vel_x = -((vel_x.abs() + 0.018).min(max_ball_speed * 0.96));
                vel_y += ((by - ai_y) as f64 / ai_paddle_h as f64 - 0.5) * 0.22;
                play_sound(state, "paddle");
            }
            if ball_x < 0.0 {
                ai_score += 1;
                play_sound(state, "score");
                reset_pong_ball(
                    &mut ball_x,
                    &mut ball_y,
                    &mut vel_x,
                    &mut vel_y,
                    board_w,
                    board_h,
                    true,
                    speed_factor,
                );
            } else if ball_x >= board_w as f64 {
                player_score += 1;
                play_sound(state, "score");
                reset_pong_ball(
                    &mut ball_x,
                    &mut ball_y,
                    &mut vel_x,
                    &mut vel_y,
                    board_w,
                    board_h,
                    false,
                    speed_factor,
                );
            }
            draw_pong(
                state,
                board_w,
                board_h,
                player_paddle_h,
                ai_paddle_h,
                player_y,
                ai_y,
                ball_x,
                ball_y,
                player_score,
                ai_score,
                win_score,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        let score = (player_score * 100 - ai_score * 25).max(0) as u32;
        record_score(state, "Pong", score);
        let result = if player_score > ai_score {
            "You won the set."
        } else {
            "CPU took the set."
        };
        if !wait_menu(
            state,
            "Pong",
            &[
                result.to_string(),
                format!("Final score: {player_score}-{ai_score}"),
                format!(
                    "Assist: {}   Speed: {}",
                    state.pong_assist_name(),
                    state.pong_speed_name()
                ),
            ],
            true,
        ) {
            return;
        }
    }
}

fn reset_pong_ball(
    ball_x: &mut f64,
    ball_y: &mut f64,
    vel_x: &mut f64,
    vel_y: &mut f64,
    w: i32,
    h: i32,
    right: bool,
    speed_factor: f64,
) {
    *ball_x = w as f64 / 2.0;
    *ball_y = h as f64 / 2.0;
    *vel_x = if right { vel_x.abs() } else { -vel_x.abs() };
    *vel_y = if *vel_y >= 0.0 {
        0.28 * speed_factor
    } else {
        -0.28 * speed_factor
    };
}

fn draw_pong(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    player_paddle_h: i32,
    ai_paddle_h: i32,
    player_y: i32,
    ai_y: i32,
    ball_x: f64,
    ball_y: f64,
    player_score: i32,
    ai_score: i32,
    win_score: i32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "PONG", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!(
            "You {player_score}   CPU {ai_score}   Target {win_score}   Assist {}   Speed {}   W/S move",
            state.pong_assist_name(),
            state.pong_speed_name()
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for y in 0..board_h {
        if y % 2 == 0 {
            put(
                &mut buf,
                top + y as usize,
                left + board_w as usize / 2,
                "|",
                &theme,
                Role::Muted,
                false,
            );
        }
    }
    for offset in 0..player_paddle_h {
        put(
            &mut buf,
            top + (player_y + offset) as usize,
            left + 1,
            "#",
            &theme,
            Role::Secondary,
            true,
        );
    }
    for offset in 0..ai_paddle_h {
        put(
            &mut buf,
            top + (ai_y + offset) as usize,
            left + board_w as usize - 2,
            "#",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + ball_y.round().clamp(0.0, (board_h - 1) as f64) as usize,
        left + ball_x.round().clamp(0.0, (board_w - 1) as f64) as usize,
        "O",
        &theme,
        Role::Success,
        true,
    );
    flush(&buf);
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum TronDir {
    Up,
    Down,
    Left,
    Right,
}

fn tron_delta(dir: TronDir) -> (i32, i32) {
    match dir {
        TronDir::Up => (0, -1),
        TronDir::Down => (0, 1),
        TronDir::Left => (-1, 0),
        TronDir::Right => (1, 0),
    }
}

fn tron_opposite(a: TronDir, b: TronDir) -> bool {
    matches!(
        (a, b),
        (TronDir::Up, TronDir::Down)
            | (TronDir::Down, TronDir::Up)
            | (TronDir::Left, TronDir::Right)
            | (TronDir::Right, TronDir::Left)
    )
}

fn tron_turn_dir(current: TronDir, key: Key) -> TronDir {
    let wanted = match key {
        Key::Up | Key::Char('w') => Some(TronDir::Up),
        Key::Down | Key::Char('s') => Some(TronDir::Down),
        Key::Left | Key::Char('a') => Some(TronDir::Left),
        Key::Right | Key::Char('d') => Some(TronDir::Right),
        _ => None,
    };
    match wanted {
        Some(dir) if !tron_opposite(current, dir) => dir,
        _ => current,
    }
}

fn tron_next_position(pos: (i32, i32), dir: TronDir) -> (i32, i32) {
    let delta = tron_delta(dir);
    (pos.0 + delta.0, pos.1 + delta.1)
}

fn tron_crashes(pos: (i32, i32), w: i32, h: i32, occupied: &HashSet<(i32, i32)>) -> bool {
    pos.0 <= 0 || pos.0 >= w - 1 || pos.1 <= 0 || pos.1 >= h - 1 || occupied.contains(&pos)
}

fn tron_safe_dirs(
    pos: (i32, i32),
    current: TronDir,
    w: i32,
    h: i32,
    occupied: &HashSet<(i32, i32)>,
) -> Vec<TronDir> {
    [TronDir::Up, TronDir::Down, TronDir::Left, TronDir::Right]
        .into_iter()
        .filter(|dir| !tron_opposite(current, *dir))
        .filter(|dir| !tron_crashes(tron_next_position(pos, *dir), w, h, occupied))
        .collect()
}

fn tron_run_length(
    pos: (i32, i32),
    dir: TronDir,
    w: i32,
    h: i32,
    occupied: &HashSet<(i32, i32)>,
) -> i32 {
    let mut count = 0;
    let mut cursor = pos;
    loop {
        cursor = tron_next_position(cursor, dir);
        if tron_crashes(cursor, w, h, occupied) {
            break;
        }
        count += 1;
    }
    count
}

fn tron_choose_cpu_dir(
    state: &mut AppState,
    pos: (i32, i32),
    current: TronDir,
    player: (i32, i32),
    w: i32,
    h: i32,
    occupied: &HashSet<(i32, i32)>,
) -> TronDir {
    let safe = tron_safe_dirs(pos, current, w, h, occupied);
    if safe.is_empty() {
        return current;
    }
    if state.difficulty_index == 0 && state.rng.chance(1, 4) {
        return safe[state.rng.usize(safe.len())];
    }
    let chase_weight = match state.difficulty_index {
        0 => 0,
        1 => 1,
        2 => 2,
        _ => 3,
    };
    let mut best = safe[0];
    let mut best_score = i32::MIN;
    for dir in safe {
        let next = tron_next_position(pos, dir);
        let distance = (next.0 - player.0).abs() + (next.1 - player.1).abs();
        let straight_bonus = if dir == current { 4 } else { 0 };
        let score = tron_run_length(pos, dir, w, h, occupied) * 5 - distance * chase_weight
            + straight_bonus
            + state.rng.range(0, 6);
        if score > best_score {
            best_score = score;
            best = dir;
        }
    }
    best
}

fn game_tron_cycles(state: &mut AppState) {
    if !require_size(state, 22, 70, "Tron Light Cycles") {
        return;
    }
    loop {
        let target_rounds = if state.endless_mode { 9 } else { 3 };
        let mut player_wins = 0;
        let mut cpu_wins = 0;
        let mut match_score = 0u32;
        let mut round = 1;
        while player_wins < target_rounds && cpu_wins < target_rounds {
            let (board_w, board_h) = full_board(60, 18, 132, 38);
            let mut player = (board_w / 4, board_h / 2);
            let mut cpu = (board_w * 3 / 4, board_h / 2);
            let mut player_dir = TronDir::Right;
            let mut cpu_dir = TronDir::Left;
            let mut player_trail = HashSet::new();
            let mut cpu_trail = HashSet::new();
            player_trail.insert(player);
            cpu_trail.insert(cpu);
            let mut tick = 0u32;
            let mut status = format!("Round {round}. First to {target_rounds}.");
            let cpu_reaction = match state.difficulty_index {
                0 => 7,
                1 => 5,
                2 => 4,
                _ => 3,
            };
            loop {
                let frame = Instant::now();
                while let Some(key) = read_key() {
                    if is_pause(key) {
                        if pause_screen(state).is_none() {
                            return;
                        }
                        continue;
                    }
                    if is_quit(key) {
                        return;
                    }
                    player_dir = tron_turn_dir(player_dir, key);
                }
                tick += 1;
                let occupied: HashSet<(i32, i32)> =
                    player_trail.union(&cpu_trail).copied().collect();
                if tick % cpu_reaction == 0 {
                    cpu_dir = tron_choose_cpu_dir(
                        state, cpu, cpu_dir, player, board_w, board_h, &occupied,
                    );
                }
                let next_player = tron_next_position(player, player_dir);
                let next_cpu = tron_next_position(cpu, cpu_dir);
                let swap_crash = next_player == cpu && next_cpu == player;
                let head_on = next_player == next_cpu || swap_crash;
                let player_crash =
                    head_on || tron_crashes(next_player, board_w, board_h, &occupied);
                let cpu_crash = head_on || tron_crashes(next_cpu, board_w, board_h, &occupied);
                if player_crash || cpu_crash {
                    if player_crash && cpu_crash {
                        status = "Both riders crashed. No point.".to_string();
                        play_sound(state, "alert");
                    } else if cpu_crash {
                        player_wins += 1;
                        match_score += 500 + tick;
                        status = "CPU crashed. You take the round.".to_string();
                        play_sound(state, "score");
                    } else {
                        cpu_wins += 1;
                        status = "You hit a trail. CPU takes the round.".to_string();
                        play_sound(state, "alert");
                    }
                    draw_tron_cycles(
                        state,
                        board_w,
                        board_h,
                        &player_trail,
                        &cpu_trail,
                        player,
                        cpu,
                        player_wins,
                        cpu_wins,
                        target_rounds,
                        &status,
                    );
                    thread::sleep(Duration::from_millis(650));
                    break;
                }
                player = next_player;
                cpu = next_cpu;
                player_trail.insert(player);
                cpu_trail.insert(cpu);
                match_score += 1;
                draw_tron_cycles(
                    state,
                    board_w,
                    board_h,
                    &player_trail,
                    &cpu_trail,
                    player,
                    cpu,
                    player_wins,
                    cpu_wins,
                    target_rounds,
                    &status,
                );
                sleep_frame(frame, state.difficulty().tick_ms + 8);
            }
            round += 1;
        }
        record_score(state, "Tron Light Cycles", match_score);
        let result = if player_wins > cpu_wins {
            "You won the grid duel."
        } else {
            "CPU owned the grid."
        };
        if !wait_menu(
            state,
            "Tron Light Cycles",
            &[
                result.to_string(),
                format!("Rounds: you {player_wins}, CPU {cpu_wins}"),
                format!("Score: {match_score}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_tron_cycles(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    player_trail: &HashSet<(i32, i32)>,
    cpu_trail: &HashSet<(i32, i32)>,
    player: (i32, i32),
    cpu: (i32, i32),
    player_wins: i32,
    cpu_wins: i32,
    target_rounds: i32,
    status: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        "TRON LIGHT CYCLES",
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!(
            "You {player_wins}   CPU {cpu_wins}   Target {target_rounds}   WASD steer   P pause   Q menu"
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in cpu_trail {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "+",
            &theme,
            Role::Danger,
            false,
        );
    }
    for &(x, y) in player_trail {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            ".",
            &theme,
            Role::Secondary,
            false,
        );
    }
    put(
        &mut buf,
        top + cpu.1 as usize,
        left + cpu.0 as usize,
        "C",
        &theme,
        Role::Danger,
        true,
    );
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize,
        "@",
        &theme,
        Role::Success,
        true,
    );
    center(
        &mut buf,
        rows - 2,
        &trim(status, cols.saturating_sub(4)),
        &theme,
        Role::Muted,
        true,
        cols,
    );
    flush(&buf);
}

fn game_tron_grid_run(state: &mut AppState) {
    if !require_size(state, 22, 70, "Tron Grid Run") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(58, 18, 132, 38);
        let goal = if state.endless_mode {
            50
        } else {
            match state.difficulty_index {
                0 => 8,
                1 => 10,
                2 => 12,
                _ => 14,
            }
        };
        let max_trail = match state.difficulty_index {
            0 => 22,
            1 => 30,
            2 => 38,
            _ => 46,
        };
        let mut player = (board_w / 2, board_h / 2);
        let mut dir = TronDir::Right;
        let mut trail = HashSet::new();
        let mut trail_order = VecDeque::new();
        trail.insert(player);
        trail_order.push_back(player);
        let mut core = tron_random_free_cell(state, board_w, board_h, &trail);
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut collected = 0;
        let mut status = "Collect cores. Your fading trail is still lethal.".to_string();
        while lives > 0 && collected < goal {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                if is_pause(key) {
                    if pause_screen(state).is_none() {
                        return;
                    }
                    continue;
                }
                if is_quit(key) {
                    return;
                }
                dir = tron_turn_dir(dir, key);
            }
            let next = tron_next_position(player, dir);
            if tron_crashes(next, board_w, board_h, &trail) {
                lives -= 1;
                status = if lives > 0 {
                    "Trail crash. Grid reset.".to_string()
                } else {
                    "Final trail crash.".to_string()
                };
                play_sound(state, "alert");
                trail.clear();
                trail_order.clear();
                player = (board_w / 2, board_h / 2);
                dir = TronDir::Right;
                trail.insert(player);
                trail_order.push_back(player);
                core = tron_random_free_cell(state, board_w, board_h, &trail);
                draw_tron_grid_run(
                    state, board_w, board_h, &trail, player, core, lives, score, collected, goal,
                    &status,
                );
                thread::sleep(Duration::from_millis(350));
                continue;
            }
            player = next;
            trail.insert(player);
            trail_order.push_back(player);
            while trail_order.len() > max_trail {
                if let Some(old) = trail_order.pop_front() {
                    if old != player {
                        trail.remove(&old);
                    }
                }
            }
            if player == core {
                collected += 1;
                score += 80 + trail_order.len() as u32;
                for _ in 0..(max_trail / 3) {
                    if let Some(old) = trail_order.pop_front() {
                        if old != player {
                            trail.remove(&old);
                        }
                    }
                }
                core = tron_random_free_cell(state, board_w, board_h, &trail);
                status = "Core captured. Trail shortened.".to_string();
                play_sound(state, "score");
            } else {
                score += 1;
            }
            draw_tron_grid_run(
                state, board_w, board_h, &trail, player, core, lives, score, collected, goal,
                &status,
            );
            sleep_frame(frame, state.difficulty().tick_ms + 12);
        }
        if collected >= goal {
            score += lives.max(0) as u32 * 150;
        }
        record_score(state, "Tron Grid Run", score);
        let result = if collected >= goal {
            "Grid route complete."
        } else {
            "Grid route failed."
        };
        if !wait_menu(
            state,
            "Tron Grid Run",
            &[
                result.to_string(),
                format!("Cores: {collected}/{goal}"),
                format!("Score: {score}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn tron_random_free_cell(
    state: &mut AppState,
    w: i32,
    h: i32,
    blocked: &HashSet<(i32, i32)>,
) -> (i32, i32) {
    for _ in 0..400 {
        let point = (state.rng.range(2, w - 3), state.rng.range(2, h - 3));
        if !blocked.contains(&point) {
            return point;
        }
    }
    for y in 1..h - 1 {
        for x in 1..w - 1 {
            let point = (x, y);
            if !blocked.contains(&point) {
                return point;
            }
        }
    }
    (w / 2, h / 2)
}

fn draw_tron_grid_run(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    trail: &HashSet<(i32, i32)>,
    player: (i32, i32),
    core: (i32, i32),
    lives: i32,
    score: u32,
    collected: i32,
    goal: i32,
    status: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        "TRON GRID RUN",
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!(
            "Score {score}   Lives {lives}   Cores {collected}/{goal}   WASD steer   P pause   Q menu"
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in trail {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            ".",
            &theme,
            Role::Secondary,
            false,
        );
    }
    put(
        &mut buf,
        top + core.1 as usize,
        left + core.0 as usize,
        "*",
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize,
        "@",
        &theme,
        Role::Title,
        true,
    );
    center(
        &mut buf,
        rows - 2,
        &trim(status, cols.saturating_sub(4)),
        &theme,
        Role::Muted,
        true,
        cols,
    );
    flush(&buf);
}

fn game_tetris(state: &mut AppState) {
    if !require_size(state, 24, 58, "Tetris") {
        return;
    }
    loop {
        let cols = 10;
        let rows = 18;
        let mut board = vec![vec![0u8; cols]; rows];
        let mut shape = state.rng.usize(7);
        let mut next_shape = state.rng.usize(7);
        let mut rot = 0usize;
        let mut px = 3i32;
        let mut py = 0i32;
        let mut score = 0u32;
        let mut lines = 0u32;
        let mut last_drop = Instant::now();
        let mut drop_ms = match state.difficulty_index {
            0 => 720,
            1 => 520,
            _ => 360,
        };
        let mut alive = true;
        while alive {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') if !piece_hits(&board, shape, rot, px - 1, py) => {
                        px -= 1
                    }
                    Key::Right | Key::Char('d') if !piece_hits(&board, shape, rot, px + 1, py) => {
                        px += 1
                    }
                    Key::Down | Key::Char('s') if !piece_hits(&board, shape, rot, px, py + 1) => {
                        py += 1
                    }
                    Key::Up | Key::Char('w') => {
                        let new_rot = (rot + 1) % 4;
                        if !piece_hits(&board, shape, new_rot, px, py) {
                            rot = new_rot;
                        }
                    }
                    Key::Space => {
                        while !piece_hits(&board, shape, rot, px, py + 1) {
                            py += 1;
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_drop.elapsed() >= Duration::from_millis(drop_ms) {
                if piece_hits(&board, shape, rot, px, py + 1) {
                    lock_piece(&mut board, shape, rot, px, py);
                    let cleared = clear_lines(&mut board);
                    if cleared > 0 {
                        lines += cleared as u32;
                        score += [0, 100, 300, 500, 800][cleared] as u32;
                        drop_ms = drop_ms.saturating_sub((cleared * 8) as u64).max(120);
                        play_sound(state, "score");
                    }
                    shape = next_shape;
                    next_shape = state.rng.usize(7);
                    rot = 0;
                    px = 3;
                    py = 0;
                    if piece_hits(&board, shape, rot, px, py) {
                        alive = false;
                    }
                } else {
                    py += 1;
                    score += 1;
                }
                last_drop = Instant::now();
            }
            draw_tetris(state, &board, shape, rot, px, py, next_shape, score, lines);
            sleep_frame(frame, 35);
        }
        record_score(state, "Tetris", score);
        if !wait_menu(
            state,
            "Tetris",
            &[
                format!("Game over. Score: {score}"),
                format!("Lines cleared: {lines}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn base_shape(shape: usize) -> [(i32, i32); 4] {
    match shape {
        0 => [(0, 1), (1, 1), (2, 1), (3, 1)],
        1 => [(0, 0), (0, 1), (1, 1), (2, 1)],
        2 => [(2, 0), (0, 1), (1, 1), (2, 1)],
        3 => [(1, 0), (2, 0), (1, 1), (2, 1)],
        4 => [(1, 0), (2, 0), (0, 1), (1, 1)],
        5 => [(1, 0), (0, 1), (1, 1), (2, 1)],
        _ => [(0, 0), (1, 0), (1, 1), (2, 1)],
    }
}

fn piece_cells(shape: usize, rot: usize, px: i32, py: i32) -> Vec<(i32, i32)> {
    base_shape(shape)
        .iter()
        .map(|&(mut x, mut y)| {
            if shape != 3 {
                for _ in 0..rot {
                    let old_x = x;
                    x = 3 - y;
                    y = old_x;
                }
            }
            (px + x, py + y)
        })
        .collect()
}

fn piece_hits(board: &[Vec<u8>], shape: usize, rot: usize, px: i32, py: i32) -> bool {
    for (x, y) in piece_cells(shape, rot, px, py) {
        if x < 0 || x >= board[0].len() as i32 || y >= board.len() as i32 {
            return true;
        }
        if y >= 0 && board[y as usize][x as usize] != 0 {
            return true;
        }
    }
    false
}

fn lock_piece(board: &mut [Vec<u8>], shape: usize, rot: usize, px: i32, py: i32) {
    for (x, y) in piece_cells(shape, rot, px, py) {
        if y >= 0 && y < board.len() as i32 && x >= 0 && x < board[0].len() as i32 {
            board[y as usize][x as usize] = (shape + 1) as u8;
        }
    }
}

fn clear_lines(board: &mut Vec<Vec<u8>>) -> usize {
    let cols = board[0].len();
    let before = board.len();
    board.retain(|row| row.iter().any(|&cell| cell == 0));
    let cleared = before - board.len();
    for _ in 0..cleared {
        board.insert(0, vec![0; cols]);
    }
    cleared
}

fn draw_tetris(
    state: &AppState,
    board: &[Vec<u8>],
    shape: usize,
    rot: usize,
    px: i32,
    py: i32,
    next_shape: usize,
    score: u32,
    lines: u32,
) {
    let (term_rows, term_cols) = terminal_size();
    let theme = state.theme().clone();
    let rows = board.len();
    let cols = board[0].len();
    let top = term_rows / 2 - rows / 2;
    let left = term_cols / 2 - 20;
    let active: HashSet<_> = piece_cells(shape, rot, px, py).into_iter().collect();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "TETRIS", &theme, Role::Title, true, term_cols);
    draw_box(
        &mut buf,
        top,
        left,
        rows + 2,
        cols * 2 + 2,
        "WELL",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for y in 0..rows {
        for x in 0..cols {
            let filled = board[y][x] != 0 || active.contains(&(x as i32, y as i32));
            put(
                &mut buf,
                top + 1 + y,
                left + 1 + x * 2,
                if filled { "[]" } else { "  " },
                &theme,
                if filled { Role::Secondary } else { Role::Muted },
                filled,
            );
        }
    }
    let info_left = left + cols * 2 + 5;
    draw_box(
        &mut buf,
        top,
        info_left,
        11,
        24,
        "STATUS",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    put(
        &mut buf,
        top + 2,
        info_left + 2,
        &format!("Score: {score}"),
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + 3,
        info_left + 2,
        &format!("Lines: {lines}"),
        &theme,
        Role::Secondary,
        false,
    );
    put(
        &mut buf,
        top + 4,
        info_left + 2,
        &format!("Next: {}", ["I", "J", "L", "O", "S", "T", "Z"][next_shape]),
        &theme,
        Role::Accent,
        false,
    );
    put(
        &mut buf,
        top + 6,
        info_left + 2,
        "Arrows/WASD move",
        &theme,
        Role::Muted,
        false,
    );
    put(
        &mut buf,
        top + 7,
        info_left + 2,
        "Up/W rotate",
        &theme,
        Role::Muted,
        false,
    );
    put(
        &mut buf,
        top + 8,
        info_left + 2,
        "Space hard drop",
        &theme,
        Role::Muted,
        false,
    );
    put(
        &mut buf,
        top + 9,
        info_left + 2,
        "Q menu",
        &theme,
        Role::Muted,
        false,
    );
    flush(&buf);
}

fn game_breakout(state: &mut AppState) {
    if !require_size(state, 22, 70, "Breakout") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(58, 17, 132, 38);
        let paddle_w = match state.difficulty_index {
            0 => 12,
            1 => 10,
            _ => 8,
        };
        let mut paddle_x = board_w / 2 - paddle_w / 2;
        let mut ball_x = board_w as f64 / 2.0;
        let mut ball_y = board_h as f64 - 4.0;
        let mut vel_x = 0.42 * state.difficulty().speed;
        let mut vel_y = -0.36 * state.difficulty().speed;
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut bricks = HashSet::new();
        for y in 1..5 {
            for x in (3..board_w - 3).step_by(5) {
                bricks.insert((x, y));
            }
        }
        while lives > 0 && !bricks.is_empty() {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => paddle_x = (paddle_x - 3).max(1),
                    Key::Right | Key::Char('d') => {
                        paddle_x = (paddle_x + 3).min(board_w - paddle_w - 1)
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            ball_x += vel_x;
            ball_y += vel_y;
            if ball_x <= 1.0 || ball_x >= (board_w - 2) as f64 {
                vel_x = -vel_x;
                play_sound(state, "wall");
            }
            if ball_y <= 1.0 {
                vel_y = vel_y.abs();
                play_sound(state, "wall");
            }
            let bx = ball_x.round() as i32;
            let by = ball_y.round() as i32;
            if by == board_h - 2 && bx >= paddle_x && bx < paddle_x + paddle_w {
                vel_y = -vel_y.abs();
                vel_x += ((bx - paddle_x) as f64 / paddle_w as f64 - 0.5) * 0.16;
                play_sound(state, "paddle");
            }
            let hit_brick = bricks
                .iter()
                .copied()
                .find(|(x, y)| by == *y && bx >= *x && bx < *x + 4);
            if let Some(brick) = hit_brick {
                bricks.remove(&brick);
                score += 25;
                vel_y = -vel_y;
                play_sound(state, "score");
            }
            if ball_y > board_h as f64 {
                lives -= 1;
                play_sound(state, "alert");
                ball_x = board_w as f64 / 2.0;
                ball_y = board_h as f64 - 4.0;
                vel_y = -vel_y.abs();
            }
            draw_breakout(
                state, board_w, board_h, paddle_w, paddle_x, ball_x, ball_y, lives, score, &bricks,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, "Breakout", score);
        let outcome = if bricks.is_empty() {
            "Board cleared."
        } else {
            "Out of lives."
        };
        if !wait_menu(
            state,
            "Breakout",
            &[outcome.to_string(), format!("Score: {score}")],
            true,
        ) {
            return;
        }
    }
}

fn draw_breakout(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    paddle_w: i32,
    paddle_x: i32,
    ball_x: f64,
    ball_y: f64,
    lives: i32,
    score: u32,
    bricks: &HashSet<(i32, i32)>,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "BREAKOUT", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   A/D move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in bricks {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "====",
            &theme,
            Role::Secondary,
            true,
        );
    }
    put(
        &mut buf,
        top + ball_y.round().clamp(0.0, (board_h - 1) as f64) as usize,
        left + ball_x.round().clamp(0.0, (board_w - 1) as f64) as usize,
        "O",
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + board_h as usize - 2,
        left + paddle_x as usize,
        &"=".repeat(paddle_w as usize),
        &theme,
        Role::Accent,
        true,
    );
    flush(&buf);
}

fn game_invaders(state: &mut AppState) {
    if !require_size(state, 24, 74, "Space Invaders") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(64, 18, 140, 38);
        let mut player_x = board_w / 2;
        let mut bullets: Vec<(i32, i32)> = Vec::new();
        let mut bombs: Vec<(i32, f64)> = Vec::new();
        let mut invaders = Vec::new();
        let alien_cols = if board_w >= 104 { 12 } else { 9 };
        let alien_rows = if board_h >= 26 { 5 } else { 4 };
        let alien_span = (alien_cols - 1) * 5 + 3;
        let alien_start = ((board_w - alien_span) / 2).max(3);
        for row in 0..alien_rows {
            for col in 0..alien_cols {
                invaders.push((alien_start + col * 5, 2 + row * 2));
            }
        }
        let mut shields = HashSet::new();
        let shield_count = if board_w >= 104 { 6 } else { 4 };
        let shield_y = (board_h - 6).max(12);
        for block in 0..shield_count {
            let center_x = ((board_w as f64) * (block as f64 + 1.0) / (shield_count as f64 + 1.0))
                .round() as i32;
            for y in shield_y..shield_y + 2 {
                for x in center_x - 3..=center_x + 3 {
                    shields.insert((x, y));
                }
            }
        }
        let mut dir = 1;
        let mut last_move = Instant::now();
        let mut last_bomb = Instant::now();
        let mut score = 0u32;
        let mut alive = true;
        while alive && !invaders.is_empty() {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => player_x = (player_x - 3).max(2),
                    Key::Right | Key::Char('d') => player_x = (player_x + 3).min(board_w - 3),
                    Key::Space
                        if bullets.len()
                            < match state.difficulty_index {
                                0 => 5,
                                1 => 4,
                                _ => 3,
                            } =>
                    {
                        bullets.push((player_x, board_h - 3));
                        play_sound(state, "paddle");
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            for bullet in &mut bullets {
                bullet.1 -= 1;
            }
            bullets.retain(|(_, y)| *y > 0);
            for bomb in &mut bombs {
                bomb.1 += 0.20 * state.difficulty().speed;
            }
            bombs.retain(|(_, y)| *y < board_h as f64);
            let move_gap =
                Duration::from_millis((640.0 / state.difficulty().speed).max(180.0) as u64);
            if last_move.elapsed() >= move_gap {
                let edge = invaders
                    .iter()
                    .any(|(x, _)| (*x >= board_w - 5 && dir > 0) || (*x <= 3 && dir < 0));
                if edge {
                    for alien in &mut invaders {
                        alien.1 += 1;
                    }
                    dir *= -1;
                } else {
                    for alien in &mut invaders {
                        alien.0 += dir;
                    }
                }
                last_move = Instant::now();
            }
            if last_bomb.elapsed()
                >= Duration::from_millis((900.0 / state.difficulty().speed) as u64)
                && !invaders.is_empty()
            {
                let alien = invaders[state.rng.usize(invaders.len())];
                bombs.push((alien.0, alien.1 as f64 + 1.0));
                last_bomb = Instant::now();
            }
            let mut hit_aliens = HashSet::new();
            let mut used_bullets = HashSet::new();
            for (bi, bullet) in bullets.iter().enumerate() {
                if shields.remove(bullet) {
                    used_bullets.insert(bi);
                }
                for (ai, alien) in invaders.iter().enumerate() {
                    if (bullet.0 - alien.0).abs() <= 1 && (bullet.1 - alien.1).abs() <= 1 {
                        hit_aliens.insert(ai);
                        used_bullets.insert(bi);
                    }
                }
            }
            if !hit_aliens.is_empty() {
                score += hit_aliens.len() as u32 * 40;
                play_sound(state, "score");
            }
            invaders = invaders
                .into_iter()
                .enumerate()
                .filter_map(|(i, alien)| (!hit_aliens.contains(&i)).then_some(alien))
                .collect();
            bullets = bullets
                .into_iter()
                .enumerate()
                .filter_map(|(i, bullet)| (!used_bullets.contains(&i)).then_some(bullet))
                .collect();
            for bomb in &bombs {
                let p = (bomb.0, bomb.1.round() as i32);
                shields.remove(&p);
                if (p.0 - player_x).abs() <= 1 && p.1 >= board_h - 3 {
                    alive = false;
                }
            }
            if invaders.iter().any(|(_, y)| *y >= board_h - 4) {
                alive = false;
            }
            draw_invaders(
                state, board_w, board_h, player_x, &bullets, &bombs, &invaders, &shields, score,
            );
            sleep_frame(frame, 45);
        }
        record_score(state, "Space Invaders", score);
        let outcome = if invaders.is_empty() {
            "Alien block cleared."
        } else {
            "The invasion landed."
        };
        if !wait_menu(
            state,
            "Space Invaders",
            &[outcome.to_string(), format!("Score: {score}")],
            true,
        ) {
            return;
        }
    }
}

fn draw_invaders(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    player_x: i32,
    bullets: &[(i32, i32)],
    bombs: &[(i32, f64)],
    invaders: &[(i32, i32)],
    shields: &HashSet<(i32, i32)>,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        "SPACE INVADERS",
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Score {score}   Shields online   A/D move   Space fire   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in invaders {
        put(
            &mut buf,
            top + y as usize,
            left + (x - 1) as usize,
            "<M>",
            &theme,
            Role::Danger,
            true,
        );
    }
    for &(x, y) in shields {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "#",
            &theme,
            Role::Secondary,
            true,
        );
    }
    for &(x, y) in bullets {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "|",
            &theme,
            Role::Success,
            true,
        );
    }
    for &(x, y) in bombs {
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + x as usize,
            "!",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + board_h as usize - 2,
        left + player_x as usize - 1,
        "/A\\",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_missile(state: &mut AppState) {
    if !require_size(state, 24, 76, "Missile Command") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(66, 18, 140, 38);
        let mut cursor = (board_w / 2, board_h / 2);
        let city_count = if board_w >= 104 { 7 } else { 5 };
        let city_xs: Vec<i32> = (0..city_count)
            .map(|index| {
                ((board_w as f64) * (index as f64 + 1.0) / (city_count as f64 + 1.0)).round() as i32
            })
            .collect();
        let mut cities = vec![true; city_xs.len()];
        let mut missiles: Vec<(f64, f64, f64, f64, usize)> = Vec::new();
        let mut explosions: Vec<(i32, i32, i32)> = Vec::new();
        let mut last_spawn = Instant::now();
        let mut score = 0u32;
        while cities.iter().any(|&city| city) {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => cursor.1 = (cursor.1 - 1).max(1),
                    Key::Down | Key::Char('s') => cursor.1 = (cursor.1 + 1).min(board_h - 3),
                    Key::Left | Key::Char('a') => cursor.0 = (cursor.0 - 2).max(1),
                    Key::Right | Key::Char('d') => cursor.0 = (cursor.0 + 2).min(board_w - 2),
                    Key::Space => {
                        explosions.push((cursor.0, cursor.1, 1));
                        play_sound(state, "paddle");
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((1050.0 / state.difficulty().speed) as u64)
            {
                let target = state.rng.usize(city_xs.len());
                let sx = state.rng.range(1, board_w - 2) as f64;
                let tx = city_xs[target] as f64;
                let ty = board_h as f64 - 2.0;
                let speed = 0.13 * state.difficulty().speed;
                let dx = tx - sx;
                let dy = ty;
                let len = (dx * dx + dy * dy).sqrt();
                missiles.push((sx, 0.0, dx / len * speed, dy / len * speed, target));
                last_spawn = Instant::now();
            }
            for missile in &mut missiles {
                missile.0 += missile.2;
                missile.1 += missile.3;
            }
            for explosion in &mut explosions {
                explosion.2 += 1;
            }
            explosions.retain(|(_, _, r)| {
                *r <= match state.difficulty_index {
                    0 => 10,
                    1 => 8,
                    _ => 6,
                }
            });
            let mut destroyed = HashSet::new();
            for (mi, missile) in missiles.iter().enumerate() {
                for &(ex, ey, r) in &explosions {
                    let dx = missile.0 - ex as f64;
                    let dy = missile.1 - ey as f64;
                    if dx * dx + dy * dy <= (r * r) as f64 {
                        destroyed.insert(mi);
                    }
                }
            }
            if !destroyed.is_empty() {
                score += destroyed.len() as u32 * 50;
                play_sound(state, "score");
            }
            let mut kept = Vec::new();
            for (i, missile) in missiles.into_iter().enumerate() {
                if destroyed.contains(&i) {
                    continue;
                }
                if missile.1 >= board_h as f64 - 2.0 {
                    if missile.4 < cities.len() {
                        cities[missile.4] = false;
                        play_sound(state, "alert");
                    }
                } else {
                    kept.push(missile);
                }
            }
            missiles = kept;
            draw_missile(
                state,
                board_w,
                board_h,
                cursor,
                &city_xs,
                &cities,
                &missiles,
                &explosions,
                score,
            );
            sleep_frame(frame, 45);
        }
        record_score(state, "Missile Command", score);
        if !wait_menu(
            state,
            "Missile Command",
            &[
                format!("All cities lost. Score: {score}"),
                "You held the line as long as you could.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_missile(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    cursor: (i32, i32),
    city_xs: &[i32],
    cities: &[bool],
    missiles: &[(f64, f64, f64, f64, usize)],
    explosions: &[(i32, i32, i32)],
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        "MISSILE COMMAND",
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Score {score}   WASD aim   Space blast   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for missile in missiles {
        put(
            &mut buf,
            top + missile.1.round().max(0.0) as usize,
            left + missile.0.round().clamp(0.0, (board_w - 1) as f64) as usize,
            "v",
            &theme,
            Role::Danger,
            true,
        );
    }
    for &(x, y, r) in explosions {
        for yy in (y - r).max(1)..=(y + r).min(board_h - 2) {
            for xx in (x - r).max(1)..=(x + r).min(board_w - 2) {
                let dx = xx - x;
                let dy = yy - y;
                if dx * dx + dy * dy <= r * r && (dx.abs() + dy.abs()) % 3 == 0 {
                    put(
                        &mut buf,
                        top + yy as usize,
                        left + xx as usize,
                        ".",
                        &theme,
                        Role::Success,
                        false,
                    );
                }
            }
        }
    }
    for (i, &x) in city_xs.iter().enumerate() {
        put(
            &mut buf,
            top + board_h as usize - 2,
            left + x as usize - 2,
            if cities[i] { "[###]" } else { "[   ]" },
            &theme,
            if cities[i] {
                Role::Secondary
            } else {
                Role::Danger
            },
            cities[i],
        );
    }
    put(
        &mut buf,
        top + cursor.1 as usize,
        left + cursor.0 as usize,
        "+",
        &theme,
        Role::Title,
        true,
    );
    flush(&buf);
}

fn game_meteor(state: &mut AppState) {
    falling_game(state, "Meteor Dodge", "/A\\", None, "*", 0, true);
}

fn game_star(state: &mut AppState) {
    falling_game(state, "Star Catcher", "[@]", Some("*"), "!", 15, false);
}

fn game_block_drop(state: &mut AppState) {
    falling_game(state, "Block Drop", "[_]", Some("[]"), "XX", 10, false);
}

#[derive(Clone, Copy)]
struct FallingRules {
    mechanic: &'static str,
    spawn_ms: u64,
    fall_speed: f64,
    good_chance: (u32, u32),
    horizontal_step: i32,
    vertical_step: i32,
    catch_width: i32,
    combo_cap: u32,
    cargo_goal_base: Option<i32>,
    oxygen_mode: bool,
    wind_period: u32,
    missed_good_penalty: u32,
    help: &'static str,
}

fn falling_rules(name: &str, difficulty_index: usize) -> FallingRules {
    match name {
        "Meteor Dodge" => FallingRules {
            mechanic: "freeflight-survival-meteors",
            spawn_ms: 420,
            fall_speed: 0.31,
            good_chance: (0, 1),
            horizontal_step: 2,
            vertical_step: 1,
            catch_width: 2,
            combo_cap: 1,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 0,
            help: "Free-fly and survive the meteor shower",
        },
        "Star Catcher" => FallingRules {
            mechanic: "bottom-star-bomb-catch",
            spawn_ms: 480,
            fall_speed: 0.28,
            good_chance: (2, 5),
            horizontal_step: 3,
            vertical_step: 0,
            catch_width: 2,
            combo_cap: 1,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 5,
            help: "Bottom catcher: stars score, missed stars sting",
        },
        "Block Drop" => FallingRules {
            mechanic: "wide-bucket-cracked-crates",
            spawn_ms: 450,
            fall_speed: 0.34,
            good_chance: (1, 2),
            horizontal_step: 2,
            vertical_step: 0,
            catch_width: 3,
            combo_cap: 1,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 2,
            help: "Wide bucket, heavier cracked blocks",
        },
        "Comet Catcher" => FallingRules {
            mechanic: "high-combo-comet-shower",
            spawn_ms: 360,
            fall_speed: 0.38,
            good_chance: (1, 3),
            horizontal_step: 2,
            vertical_step: 1,
            catch_width: 2,
            combo_cap: 9,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 7,
            help: "Free-fly catches stack a comet combo",
        },
        "Cargo Catch" => FallingRules {
            mechanic: "quota-loader-bottom-bucket",
            spawn_ms: 390,
            fall_speed: 0.32,
            good_chance: (1, 2),
            horizontal_step: 2,
            vertical_step: 0,
            catch_width: 3,
            combo_cap: 1,
            cargo_goal_base: Some(match difficulty_index {
                0 => 8,
                1 => 11,
                _ => 14,
            }),
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 4,
            help: "Load the quota before cracked cargo breaks you",
        },
        "Gem Rush" => FallingRules {
            mechanic: "tight-gem-chain-rush",
            spawn_ms: 320,
            fall_speed: 0.45,
            good_chance: (2, 5),
            horizontal_step: 2,
            vertical_step: 0,
            catch_width: 1,
            combo_cap: 12,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 8,
            help: "Narrow bucket, fast gems, brutal combo resets",
        },
        "Pearl Diver" => FallingRules {
            mechanic: "oxygen-dive-free-swim",
            spawn_ms: 440,
            fall_speed: 0.29,
            good_chance: (1, 3),
            horizontal_step: 2,
            vertical_step: 1,
            catch_width: 2,
            combo_cap: 1,
            cargo_goal_base: None,
            oxygen_mode: true,
            wind_period: 0,
            missed_good_penalty: 0,
            help: "Free-swim for pearls; oxygen is the real timer",
        },
        "Data Storm" => FallingRules {
            mechanic: "narrow-data-packet-streaks",
            spawn_ms: 290,
            fall_speed: 0.48,
            good_chance: (1, 2),
            horizontal_step: 1,
            vertical_step: 0,
            catch_width: 1,
            combo_cap: 6,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 10,
            help: "Tiny moves, fast packets, errors break streaks",
        },
        "Rain Runner" => FallingRules {
            mechanic: "windy-umbrella-rain-combo",
            spawn_ms: 370,
            fall_speed: 0.37,
            good_chance: (2, 5),
            horizontal_step: 2,
            vertical_step: 0,
            catch_width: 4,
            combo_cap: 5,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 7,
            missed_good_penalty: 3,
            help: "Wide umbrella, gusts shove you sideways",
        },
        _ => FallingRules {
            mechanic: "fallback-falling",
            spawn_ms: 520,
            fall_speed: 0.25,
            good_chance: (2, 5),
            horizontal_step: 2,
            vertical_step: 1,
            catch_width: 2,
            combo_cap: 1,
            cargo_goal_base: None,
            oxygen_mode: false,
            wind_period: 0,
            missed_good_penalty: 3,
            help: "Catch good objects and dodge hazards",
        },
    }
}

fn falling_game(
    state: &mut AppState,
    name: &str,
    player_sprite: &str,
    good_sprite: Option<&str>,
    bad_sprite: &str,
    good_value: u32,
    survival_score: bool,
) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    let rules = falling_rules(name, state.difficulty_index);
    let combo_mode = matches!(
        name,
        "Comet Catcher" | "Gem Rush" | "Data Storm" | "Rain Runner"
    );
    let cargo_goal = rules.cargo_goal_base;
    let oxygen_mode = rules.oxygen_mode;
    loop {
        let (board_w, board_h) = full_board(52, 17, 132, 38);
        let mut player = (board_w / 2, board_h - 2);
        let mut objects: Vec<(i32, f64, bool)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut combo = 1u32;
        let mut cargo = 0i32;
        let mut oxygen = 100i32;
        let mut completed = false;
        let mut tick = 0u32;
        let mut last_spawn = Instant::now();
        let mut oxygen_tick = Instant::now();
        while lives > 0 && !completed {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => {
                        player.0 = (player.0 - rules.horizontal_step).max(1)
                    }
                    Key::Right | Key::Char('d') => {
                        player.0 = (player.0 + rules.horizontal_step).min(board_w - 3)
                    }
                    Key::Up | Key::Char('w') if rules.vertical_step > 0 => {
                        player.1 = (player.1 - rules.vertical_step).max(1)
                    }
                    Key::Down | Key::Char('s') if rules.vertical_step > 0 => {
                        player.1 = (player.1 + rules.vertical_step).min(board_h - 2)
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            tick += 1;
            if rules.wind_period > 0 && tick % rules.wind_period == 0 {
                let push = if (tick / rules.wind_period) % 2 == 0 {
                    1
                } else {
                    -1
                };
                player.0 = (player.0 + push).clamp(1, board_w - 3);
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((rules.spawn_ms as f64 / state.difficulty().speed) as u64)
            {
                let good = good_sprite.is_some()
                    && state.rng.chance(rules.good_chance.0, rules.good_chance.1);
                objects.push((state.rng.range(2, board_w - 4), 1.0, good));
                if state.difficulty_index > 0 && state.rng.chance(1, 5) {
                    objects.push((state.rng.range(2, board_w - 4), 1.0, false));
                }
                last_spawn = Instant::now();
            }
            for object in &mut objects {
                object.1 += rules.fall_speed * state.difficulty().speed;
            }
            if oxygen_mode && oxygen_tick.elapsed() >= Duration::from_millis(350) {
                oxygen -= 1 + state.difficulty_index as i32;
                oxygen_tick = Instant::now();
                if oxygen <= 0 {
                    lives -= 1;
                    oxygen = 100;
                    play_sound(state, "alert");
                }
            }
            let mut kept = Vec::new();
            for object in objects.into_iter() {
                let oy = object.1.round() as i32;
                if oy >= board_h {
                    if object.2 && !survival_score {
                        score = score.saturating_sub(rules.missed_good_penalty);
                        if combo_mode {
                            combo = 1;
                        }
                        if oxygen_mode {
                            oxygen = (oxygen - 6).max(0);
                        }
                    }
                    continue;
                }
                if oy == player.1 && (object.0 - player.0).abs() <= rules.catch_width {
                    if object.2 {
                        if let Some(goal) = cargo_goal {
                            cargo += 1;
                            score += good_value;
                            if cargo >= goal {
                                completed = true;
                                score += lives.max(0) as u32 * 100;
                            }
                        } else if oxygen_mode {
                            oxygen = (oxygen + 14).min(100);
                            score += good_value;
                        } else if combo_mode {
                            score += good_value * combo;
                            combo = (combo + 1).min(rules.combo_cap.max(1));
                        } else {
                            score += good_value;
                        }
                        play_sound(state, "score");
                    } else {
                        lives -= 1;
                        combo = 1;
                        if oxygen_mode {
                            oxygen = (oxygen - 20).max(0);
                        }
                        play_sound(state, "alert");
                    }
                } else {
                    kept.push(object);
                }
            }
            objects = kept;
            if survival_score {
                score += 1;
            }
            let status = if let Some(goal) = cargo_goal {
                format!("Cargo {cargo}/{goal}")
            } else if oxygen_mode {
                format!("O2 {oxygen}")
            } else if combo_mode {
                format!("Combo x{combo}")
            } else if survival_score {
                rules.mechanic.to_string()
            } else {
                rules.help.to_string()
            };
            draw_falling(
                state,
                name,
                board_w,
                board_h,
                player,
                player_sprite,
                good_sprite,
                bad_sprite,
                &objects,
                lives,
                score,
                &status,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, name, score);
        let lines = if completed {
            vec![
                format!("Goal complete. Score: {score}"),
                "Clean run bonus added for remaining lives.".to_string(),
            ]
        } else if oxygen_mode {
            vec![
                format!("Dive over. Score: {score}"),
                "Pearls refill oxygen. Hazards and missed pearls drain it.".to_string(),
            ]
        } else if combo_mode {
            vec![
                format!("Run over. Score: {score}"),
                "Catches build combo. Misses and hazards reset it.".to_string(),
            ]
        } else {
            vec![
                format!("Run over. Score: {score}"),
                "Lives reached zero.".to_string(),
            ]
        };
        if !wait_menu(state, name, &lines, true) {
            return;
        }
    }
}

fn draw_falling(
    state: &AppState,
    name: &str,
    board_w: i32,
    board_h: i32,
    player: (i32, i32),
    player_sprite: &str,
    good_sprite: Option<&str>,
    bad_sprite: &str,
    objects: &[(i32, f64, bool)],
    lives: i32,
    score: u32,
    status: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    let status_text = if status.is_empty() {
        String::new()
    } else {
        format!("   {status}")
    };
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}{status_text}   WASD move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y, good) in objects {
        let sprite = if good {
            good_sprite.unwrap_or("*")
        } else {
            bad_sprite
        };
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + x as usize,
            sprite,
            &theme,
            if good { Role::Success } else { Role::Danger },
            true,
        );
    }
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize - player_sprite.len() / 2,
        player_sprite,
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_racer(state: &mut AppState) {
    if !require_size(state, 22, 54, "Racer") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(42, 17, 80, 40);
        let lane_count = if board_w >= 70 {
            6
        } else if board_w >= 55 {
            5
        } else {
            4
        };
        let lanes: Vec<i32> = (0..lane_count)
            .map(|index| {
                ((board_w as f64) * (index as f64 + 1.0) / (lane_count as f64 + 1.0)).round() as i32
            })
            .collect();
        let mut player_lane = 1usize;
        let mut obstacles: Vec<(usize, f64)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut last_spawn = Instant::now();
        while lives > 0 {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') if player_lane > 0 => player_lane -= 1,
                    Key::Right | Key::Char('d') if player_lane + 1 < lanes.len() => {
                        player_lane += 1
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((520.0 / state.difficulty().speed) as u64)
            {
                obstacles.push((state.rng.usize(lanes.len()), 1.0));
                last_spawn = Instant::now();
            }
            for obstacle in &mut obstacles {
                obstacle.1 += 0.42 * state.difficulty().speed;
            }
            let mut kept = Vec::new();
            for obstacle in obstacles.into_iter() {
                let y = obstacle.1.round() as i32;
                if y >= board_h - 2 {
                    if obstacle.0 == player_lane {
                        lives -= 1;
                        play_sound(state, "alert");
                    } else {
                        score += 10;
                    }
                } else {
                    kept.push(obstacle);
                }
            }
            obstacles = kept;
            draw_racer(
                state,
                board_w,
                board_h,
                &lanes,
                player_lane,
                &obstacles,
                lives,
                score,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, "Racer", score);
        if !wait_menu(
            state,
            "Racer",
            &[
                format!("Road closed. Score: {score}"),
                "Traffic got the better of you.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_racer(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    lanes: &[i32],
    player_lane: usize,
    obstacles: &[(usize, f64)],
    lives: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "RACER", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   A/D lanes   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &lane_x in lanes {
        for y in (0..board_h).step_by(2) {
            put(
                &mut buf,
                top + y as usize,
                left + lane_x as usize,
                "|",
                &theme,
                Role::Muted,
                false,
            );
        }
    }
    for &(lane, y) in obstacles {
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + lanes[lane] as usize - 1,
            "[X]",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + board_h as usize - 2,
        left + lanes[player_lane] as usize - 1,
        "/A\\",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_flappy(state: &mut AppState) {
    if !require_size(state, 22, 62, "Flappy Dash") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(56, 17, 132, 38);
        let player_x = 8;
        let mut player_y = board_h / 2;
        let mut gates: Vec<(f64, i32)> =
            vec![(board_w as f64 - 2.0, state.rng.range(4, board_h - 5))];
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        while lives > 0 {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => player_y = (player_y - 1).max(1),
                    Key::Down | Key::Char('s') => player_y = (player_y + 1).min(board_h - 2),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            for gate in &mut gates {
                gate.0 -= 0.58 * state.difficulty().speed;
            }
            if gates
                .last()
                .is_none_or(|gate| gate.0 < board_w as f64 - 18.0)
            {
                gates.push((board_w as f64 - 2.0, state.rng.range(4, board_h - 5)));
            }
            let mut kept = Vec::new();
            for gate in gates.into_iter() {
                let gx = gate.0.round() as i32;
                if gx < 1 {
                    score += 20;
                    play_sound(state, "score");
                } else {
                    if (gx - player_x).abs() <= 1 && (player_y - gate.1).abs() > 2 {
                        lives -= 1;
                        play_sound(state, "alert");
                    }
                    kept.push(gate);
                }
            }
            gates = kept;
            draw_flappy(
                state, board_w, board_h, player_x, player_y, &gates, lives, score,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, "Flappy Dash", score);
        if !wait_menu(
            state,
            "Flappy Dash",
            &[
                format!("Crash. Score: {score}"),
                "Direct up/down controls, no flap physics.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_flappy(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    player_x: i32,
    player_y: i32,
    gates: &[(f64, i32)],
    lives: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "FLAPPY DASH", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   Up/Down move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, gap) in gates {
        let gx = x.round() as i32;
        if gx > 0 && gx < board_w {
            for y in 1..board_h - 1 {
                if (y - gap).abs() > 2 {
                    put(
                        &mut buf,
                        top + y as usize,
                        left + gx as usize,
                        "|",
                        &theme,
                        Role::Danger,
                        true,
                    );
                }
            }
        }
    }
    put(
        &mut buf,
        top + player_y as usize,
        left + player_x as usize,
        ">",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

#[derive(Clone, Copy)]
struct ScrollRules {
    mechanic: &'static str,
    spawn_ms: u64,
    scroll_speed: f64,
    good_chance: (u32, u32),
    collision_x: i32,
    collision_y: i32,
    fuel_drain_period: u32,
    near_miss_score: u32,
    drone_shield: bool,
    vertical_wobble: bool,
    help: &'static str,
}

fn scroll_rules(name: &str) -> ScrollRules {
    match name {
        "Asteroid Belt" => ScrollRules {
            mechanic: "pure-asteroid-threading",
            spawn_ms: 410,
            scroll_speed: 0.66,
            good_chance: (0, 1),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 0,
            near_miss_score: 6,
            drone_shield: false,
            vertical_wobble: false,
            help: "Thread rocks for near-miss points",
        },
        "River Raid" => ScrollRules {
            mechanic: "fuel-river-raid",
            spawn_ms: 440,
            scroll_speed: 0.62,
            good_chance: (1, 4),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 1,
            near_miss_score: 4,
            drone_shield: false,
            vertical_wobble: false,
            help: "Fuel pickups keep the river run alive",
        },
        "Neon Drift" => ScrollRules {
            mechanic: "momentum-heat-drift",
            spawn_ms: 380,
            scroll_speed: 0.70,
            good_chance: (1, 4),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 1,
            near_miss_score: 7,
            drone_shield: false,
            vertical_wobble: false,
            help: "Drift builds heat; boosts cool the engine",
        },
        "Drone Dodge" => ScrollRules {
            mechanic: "shielded-wobble-clouds",
            spawn_ms: 360,
            scroll_speed: 0.58,
            good_chance: (0, 1),
            collision_x: 1,
            collision_y: 1,
            fuel_drain_period: 0,
            near_miss_score: 9,
            drone_shield: true,
            vertical_wobble: true,
            help: "Small drone, drifting clouds, shield absorbs hits",
        },
        "Solar Sailer" => ScrollRules {
            mechanic: "solar-charge-fuel-economy",
            spawn_ms: 430,
            scroll_speed: 0.58,
            good_chance: (1, 3),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 2,
            near_miss_score: 5,
            drone_shield: false,
            vertical_wobble: false,
            help: "Charge slows fuel drain and boosts scoring",
        },
        "Fuel Run" => ScrollRules {
            mechanic: "tight-fuel-slalom",
            spawn_ms: 350,
            scroll_speed: 0.74,
            good_chance: (1, 5),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 1,
            near_miss_score: 8,
            drone_shield: false,
            vertical_wobble: false,
            help: "Fast route, scarce fuel, high near-miss score",
        },
        "Spark Chase" => ScrollRules {
            mechanic: "spark-collection-weave",
            spawn_ms: 330,
            scroll_speed: 0.69,
            good_chance: (1, 3),
            collision_x: 1,
            collision_y: 1,
            fuel_drain_period: 2,
            near_miss_score: 6,
            drone_shield: false,
            vertical_wobble: true,
            help: "Sparks wobble; weave through for charge",
        },
        "Orbital Courier" => ScrollRules {
            mechanic: "delivery-quota-route",
            spawn_ms: 410,
            scroll_speed: 0.64,
            good_chance: (1, 3),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 1,
            near_miss_score: 5,
            drone_shield: false,
            vertical_wobble: false,
            help: "Hit packet quota for a route completion bonus",
        },
        "Storm Surge" => ScrollRules {
            mechanic: "current-pushed-surf",
            spawn_ms: 390,
            scroll_speed: 0.78,
            good_chance: (1, 4),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 1,
            near_miss_score: 6,
            drone_shield: false,
            vertical_wobble: false,
            help: "The current shoves you while fuel drains",
        },
        _ => ScrollRules {
            mechanic: "fallback-scroll",
            spawn_ms: 430,
            scroll_speed: 0.64,
            good_chance: (1, 4),
            collision_x: 2,
            collision_y: 1,
            fuel_drain_period: 1,
            near_miss_score: 5,
            drone_shield: false,
            vertical_wobble: false,
            help: "Dodge hazards and grab supplies",
        },
    }
}

fn game_side_scroll(
    state: &mut AppState,
    name: &str,
    player_sprite: &str,
    bad_sprite: &str,
    good_sprite: Option<&str>,
    help: &str,
) {
    if !require_size(state, 22, 64, name) {
        return;
    }
    let rules = scroll_rules(name);
    let drift_mode = name == "Neon Drift";
    let solar_mode = name == "Solar Sailer";
    let storm_mode = name == "Storm Surge";
    let courier_goal = if name == "Orbital Courier" {
        Some(match state.difficulty_index {
            0 => 5,
            1 => 7,
            _ => 9,
        })
    } else {
        None
    };
    loop {
        let (board_w, board_h) = full_board(58, 17, 132, 38);
        let mut player = (8, board_h / 2);
        let mut objects: Vec<(f64, i32, bool)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut fuel = 80i32;
        let mut heat = 0i32;
        let mut charge = 40i32;
        let mut deliveries = 0i32;
        let mut drift_y = 0i32;
        let mut current_clock = 0i32;
        let mut shield = if rules.drone_shield { 2 } else { 0 };
        let mut completed = false;
        let mut last_spawn = Instant::now();
        while lives > 0 && fuel > 0 && !completed {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                if drift_mode {
                    match key {
                        Key::Up | Key::Char('w') => {
                            drift_y = (drift_y - 1).max(-3);
                            heat = (heat + 2).min(120);
                        }
                        Key::Down | Key::Char('s') => {
                            drift_y = (drift_y + 1).min(3);
                            heat = (heat + 2).min(120);
                        }
                        Key::Left | Key::Char('a') => player.0 = (player.0 - 1).max(2),
                        Key::Right | Key::Char('d') => player.0 = (player.0 + 1).min(board_w / 2),
                        _ if is_pause(key) => {
                            if pause_screen(state).is_none() {
                                return;
                            }
                        }
                        _ if is_quit(key) => return,
                        _ => {}
                    }
                } else {
                    match key {
                        Key::Up | Key::Char('w') => player.1 = (player.1 - 1).max(1),
                        Key::Down | Key::Char('s') => player.1 = (player.1 + 1).min(board_h - 2),
                        Key::Left | Key::Char('a') => player.0 = (player.0 - 1).max(2),
                        Key::Right | Key::Char('d') => player.0 = (player.0 + 1).min(board_w / 2),
                        _ if is_pause(key) => {
                            if pause_screen(state).is_none() {
                                return;
                            }
                        }
                        _ if is_quit(key) => return,
                        _ => {}
                    }
                }
            }
            current_clock += 1;
            if drift_mode {
                player.1 = (player.1 + drift_y).clamp(1, board_h - 2);
                if drift_y != 0 {
                    heat = (heat + drift_y.abs()).min(120);
                }
                if current_clock % 3 == 0 {
                    drift_y -= drift_y.signum();
                }
                if heat >= 100 {
                    lives -= 1;
                    heat = 45;
                    play_sound(state, "alert");
                }
            }
            if storm_mode && current_clock % 6 == 0 {
                let push = if (current_clock / 24) % 2 == 0 { -1 } else { 1 };
                player.1 = (player.1 + push).clamp(1, board_h - 2);
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((rules.spawn_ms as f64 / state.difficulty().speed) as u64)
            {
                let good = if good_sprite.is_none() {
                    false
                } else if courier_goal.is_some() || solar_mode {
                    state.rng.chance(rules.good_chance.0, rules.good_chance.1)
                } else {
                    state.rng.chance(rules.good_chance.0, rules.good_chance.1)
                };
                objects.push((board_w as f64 - 2.0, state.rng.range(1, board_h - 2), good));
                last_spawn = Instant::now();
            }
            for object in &mut objects {
                object.0 -= rules.scroll_speed * state.difficulty().speed;
                if rules.vertical_wobble && current_clock % 5 == 0 {
                    let wobble = if (object.0.round() as i32 + current_clock) % 2 == 0 {
                        1
                    } else {
                        -1
                    };
                    object.1 = (object.1 + wobble).clamp(1, board_h - 2);
                }
            }
            let mut kept = Vec::new();
            for object in objects.into_iter() {
                let ox = object.0.round() as i32;
                if ox < 1 {
                    if !object.2 {
                        score += if (object.1 - player.1).abs() <= rules.collision_y + 1 {
                            rules.near_miss_score
                        } else {
                            rules.near_miss_score / 2
                        };
                    }
                    continue;
                }
                if (ox - player.0).abs() <= rules.collision_x
                    && (object.1 - player.1).abs() <= rules.collision_y
                {
                    if object.2 {
                        if let Some(goal) = courier_goal {
                            deliveries += 1;
                            fuel = (fuel + 18).min(100);
                            score += 30;
                            if deliveries >= goal {
                                completed = true;
                                score += fuel.max(0) as u32 + lives.max(0) as u32 * 80;
                            }
                        } else if solar_mode {
                            charge = (charge + 24).min(100);
                            fuel = (fuel + 12).min(100);
                            score += 15 + charge.max(0) as u32 / 10;
                        } else if drift_mode {
                            heat = (heat - 25).max(0);
                            fuel = (fuel + 18).min(100);
                            score += 20;
                        } else {
                            fuel = (fuel + 25).min(100);
                            score += 15;
                        }
                        play_sound(state, "score");
                    } else {
                        if shield > 0 {
                            shield -= 1;
                        } else {
                            lives -= 1;
                        }
                        if drift_mode {
                            heat = (heat + 25).min(120);
                        }
                        if solar_mode {
                            charge = (charge - 30).max(0);
                        }
                        play_sound(state, "alert");
                    }
                } else {
                    kept.push(object);
                }
            }
            objects = kept;
            score += if solar_mode && charge >= 100 { 2 } else { 1 };
            if solar_mode && current_clock % 5 == 0 {
                charge = (charge - 1).max(0);
            }
            fuel -= if good_sprite.is_some()
                && rules.fuel_drain_period > 0
                && current_clock % rules.fuel_drain_period as i32 == 0
                && !(solar_mode && charge >= 85)
            {
                1
            } else {
                0
            };
            let status = if drift_mode {
                format!("Heat {heat}   Drift {drift_y}")
            } else if solar_mode {
                format!("Charge {charge}")
            } else if let Some(goal) = courier_goal {
                format!("Deliveries {deliveries}/{goal}")
            } else if storm_mode {
                let dir = if (current_clock / 24) % 2 == 0 {
                    "up"
                } else {
                    "down"
                };
                format!("Current {dir}")
            } else if rules.drone_shield {
                format!("Shield {shield}   {}", rules.mechanic)
            } else {
                rules.help.to_string()
            };
            draw_side_scroll(
                state,
                name,
                board_w,
                board_h,
                player,
                player_sprite,
                bad_sprite,
                good_sprite,
                help,
                &objects,
                lives,
                fuel,
                score,
                &status,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, name, score);
        let lines = if completed {
            vec![
                format!("Route complete. Score: {score}"),
                "Delivery bonus added for fuel and remaining lives.".to_string(),
            ]
        } else {
            vec![
                format!("Flight ended. Score: {score}"),
                format!("Lives {lives}   Fuel {fuel}"),
            ]
        };
        if !wait_menu(state, name, &lines, true) {
            return;
        }
    }
}

fn draw_side_scroll(
    state: &AppState,
    name: &str,
    board_w: i32,
    board_h: i32,
    player: (i32, i32),
    player_sprite: &str,
    bad_sprite: &str,
    good_sprite: Option<&str>,
    help: &str,
    objects: &[(f64, i32, bool)],
    lives: i32,
    fuel: i32,
    score: u32,
    status: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    let status_text = if status.is_empty() {
        String::new()
    } else {
        format!("   {status}")
    };
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   Fuel {fuel}{status_text}   {help}   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y, good) in objects {
        put(
            &mut buf,
            top + y as usize,
            left + x.round().max(0.0) as usize,
            if good {
                good_sprite.unwrap_or("F")
            } else {
                bad_sprite
            },
            &theme,
            if good { Role::Success } else { Role::Danger },
            true,
        );
    }
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize - player_sprite.len() / 2,
        player_sprite,
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_frog(state: &mut AppState) {
    if !require_size(state, 22, 58, "Frog Cross") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(50, 15, 118, 34);
        let mut frog = (board_w / 2, board_h - 1);
        let mut tick = 0i32;
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let lanes: Vec<i32> = (3..(board_h - 2)).step_by(2).collect();
        while lives > 0 {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => frog.1 = (frog.1 - 1).max(0),
                    Key::Down | Key::Char('s') => frog.1 = (frog.1 + 1).min(board_h - 1),
                    Key::Left | Key::Char('a') => frog.0 = (frog.0 - 2).max(1),
                    Key::Right | Key::Char('d') => frog.0 = (frog.0 + 2).min(board_w - 2),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            tick += 1;
            let cars = frog_cars(board_w, &lanes, tick, state.difficulty_index);
            if cars
                .iter()
                .any(|&(x, y)| y == frog.1 && frog.0 >= x && frog.0 < x + 4)
            {
                lives -= 1;
                frog = (board_w / 2, board_h - 1);
                play_sound(state, "alert");
            }
            if frog.1 == 0 {
                score += 100;
                frog = (board_w / 2, board_h - 1);
                play_sound(state, "score");
            }
            draw_frog(state, board_w, board_h, frog, &cars, lives, score);
            sleep_frame(frame, state.difficulty().tick_ms + 20);
        }
        record_score(state, "Frog Cross", score);
        if !wait_menu(
            state,
            "Frog Cross",
            &[
                format!("Splash. Score: {score}"),
                "Traffic does not yield.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn frog_cars(board_w: i32, lanes: &[i32], tick: i32, difficulty_index: usize) -> Vec<(i32, i32)> {
    let mut cars = Vec::new();
    for (i, &y) in lanes.iter().enumerate() {
        let dir = if i % 2 == 0 { 1 } else { -1 };
        let speed = 1 + difficulty_index as i32;
        let spacing = 13 - difficulty_index as i32;
        for n in 0..5 {
            let raw = n * spacing + (tick / speed) * dir + i as i32 * 4;
            let x = raw.rem_euclid(board_w + 8) - 4;
            cars.push((x, y));
        }
    }
    cars
}

fn draw_frog(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    frog: (i32, i32),
    cars: &[(i32, i32)],
    lives: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "FROG CROSS", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   WASD hop   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    put(
        &mut buf,
        top,
        left + 1,
        &"~".repeat((board_w - 2) as usize),
        &theme,
        Role::Success,
        false,
    );
    for &(x, y) in cars {
        if x > 0 && x < board_w - 3 {
            put(
                &mut buf,
                top + y as usize,
                left + x as usize,
                "####",
                &theme,
                Role::Danger,
                true,
            );
        }
    }
    put(
        &mut buf,
        top + frog.1 as usize,
        left + frog.0 as usize,
        "@",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_target(state: &mut AppState, name: &str, whack: bool) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(52, 16, 132, 36);
        let mut cursor = (board_w / 2, board_h / 2);
        let mut target = (
            state.rng.range(2, board_w - 3),
            state.rng.range(2, board_h - 3),
        );
        let mut score = 0u32;
        let mut misses = 0u32;
        let duration = match state.difficulty_index {
            0 => 45,
            1 => 35,
            _ => 25,
        };
        let mut end_at = Instant::now() + Duration::from_secs(duration);
        let mut next_target =
            Instant::now() + Duration::from_millis(if whack { 1200 } else { 5000 });
        while Instant::now() < end_at {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => cursor.1 = (cursor.1 - 1).max(1),
                    Key::Down | Key::Char('s') => cursor.1 = (cursor.1 + 1).min(board_h - 2),
                    Key::Left | Key::Char('a') => cursor.0 = (cursor.0 - 2).max(1),
                    Key::Right | Key::Char('d') => cursor.0 = (cursor.0 + 2).min(board_w - 2),
                    Key::Space => {
                        if (cursor.0 - target.0).abs() <= if whack { 1 } else { 2 }
                            && (cursor.1 - target.1).abs() <= 1
                        {
                            score += if whack { 20 } else { 25 };
                            target = (
                                state.rng.range(2, board_w - 3),
                                state.rng.range(2, board_h - 3),
                            );
                            next_target = Instant::now()
                                + Duration::from_millis(if whack { 900 } else { 5000 });
                            play_sound(state, "score");
                        } else {
                            misses += 1;
                        }
                    }
                    _ if is_pause(key) => {
                        if let Some(paused) = pause_screen(state) {
                            end_at += paused;
                            next_target += paused;
                        } else {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if Instant::now() >= next_target {
                target = (
                    state.rng.range(2, board_w - 3),
                    state.rng.range(2, board_h - 3),
                );
                next_target =
                    Instant::now() + Duration::from_millis(if whack { 900 } else { 5000 });
            }
            let remaining = end_at.saturating_duration_since(Instant::now()).as_secs();
            draw_target(
                state, name, board_w, board_h, cursor, target, score, misses, remaining, whack,
            );
            sleep_frame(frame, 45);
        }
        let final_score = score.saturating_sub(misses * 5);
        record_score(state, name, final_score);
        if !wait_menu(
            state,
            name,
            &[
                format!("Time. Score: {final_score}"),
                format!("Misses: {misses}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_target(
    state: &AppState,
    name: &str,
    board_w: i32,
    board_h: i32,
    cursor: (i32, i32),
    target: (i32, i32),
    score: u32,
    misses: u32,
    remaining: u64,
    whack: bool,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!(
            "Score {score}   Misses {misses}   Time {remaining}s   Space {}   Q menu",
            if whack { "whack" } else { "fire" }
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    if whack {
        for y in [3, 6, 9, 12] {
            for x in (6..board_w - 4).step_by(10) {
                put(
                    &mut buf,
                    top + y as usize,
                    left + x as usize - 1,
                    "(_)",
                    &theme,
                    Role::Muted,
                    false,
                );
            }
        }
        put(
            &mut buf,
            top + target.1 as usize,
            left + target.0 as usize - 1,
            "\\M/",
            &theme,
            Role::Danger,
            true,
        );
    } else {
        put(
            &mut buf,
            top + target.1 as usize,
            left + target.0 as usize - 1,
            "(*)",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + cursor.1 as usize,
        left + cursor.0 as usize,
        "+",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_pixel_pop(state: &mut AppState) {
    if !require_size(state, 22, 62, "Pixel Pop") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(52, 16, 100, 34);
        let grid_w = (board_w / 2).max(18);
        let grid_h = board_h;
        let colors = match state.difficulty_index {
            0 => 4,
            1 => 5,
            _ => 6,
        };
        let mut grid = vec![vec![0u8; grid_w as usize]; grid_h as usize];
        for y in 0..grid_h as usize {
            for x in 0..grid_w as usize {
                grid[y][x] = state.rng.usize(colors) as u8;
            }
        }
        let mut cursor = (grid_w / 2, grid_h / 2);
        let mut score = 0u32;
        let mut misses = 0u32;
        let duration = match state.difficulty_index {
            0 => 60,
            1 => 45,
            _ => 35,
        };
        let mut end_at = Instant::now() + Duration::from_secs(duration);
        while Instant::now() < end_at {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => cursor.1 = (cursor.1 - 1).max(0),
                    Key::Down | Key::Char('s') => cursor.1 = (cursor.1 + 1).min(grid_h - 1),
                    Key::Left | Key::Char('a') => cursor.0 = (cursor.0 - 1).max(0),
                    Key::Right | Key::Char('d') => cursor.0 = (cursor.0 + 1).min(grid_w - 1),
                    Key::Space | Key::Enter => {
                        let target = grid[cursor.1 as usize][cursor.0 as usize];
                        let mut queue = VecDeque::new();
                        let mut cluster = HashSet::new();
                        queue.push_back(cursor);
                        cluster.insert(cursor);
                        while let Some((x, y)) = queue.pop_front() {
                            for next in [(x + 1, y), (x - 1, y), (x, y + 1), (x, y - 1)] {
                                if next.0 >= 0
                                    && next.0 < grid_w
                                    && next.1 >= 0
                                    && next.1 < grid_h
                                    && grid[next.1 as usize][next.0 as usize] == target
                                    && cluster.insert(next)
                                {
                                    queue.push_back(next);
                                }
                            }
                        }
                        if cluster.len() >= 2 {
                            let popped = cluster.len() as u32;
                            score += popped * popped;
                            for &(x, y) in &cluster {
                                grid[y as usize][x as usize] = u8::MAX;
                            }
                            for x in 0..grid_w as usize {
                                let mut kept = Vec::new();
                                for y in (0..grid_h as usize).rev() {
                                    if grid[y][x] != u8::MAX {
                                        kept.push(grid[y][x]);
                                    }
                                }
                                let mut write = grid_h as usize;
                                for value in kept {
                                    write -= 1;
                                    grid[write][x] = value;
                                }
                                while write > 0 {
                                    write -= 1;
                                    grid[write][x] = state.rng.usize(colors) as u8;
                                }
                            }
                            play_sound(state, "score");
                        } else {
                            misses += 1;
                            score = score.saturating_sub(5);
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if let Some(paused) = pause_screen(state) {
                            end_at += paused;
                        } else {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            let remaining = end_at.saturating_duration_since(Instant::now()).as_secs();
            draw_pixel_pop(
                state, board_w, board_h, grid_w, grid_h, &grid, cursor, score, misses, remaining,
            );
            sleep_frame(frame, 45);
        }
        let final_score = score.saturating_sub(misses * 3);
        record_score(state, "Pixel Pop", final_score);
        if !wait_menu(
            state,
            "Pixel Pop",
            &[
                format!("Time. Score: {final_score}"),
                "Pop connected color clusters. Bigger clusters pay more.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_pixel_pop(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    grid_w: i32,
    grid_h: i32,
    grid: &[Vec<u8>],
    cursor: (i32, i32),
    score: u32,
    misses: u32,
    remaining: u64,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "PIXEL POP", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!(
            "Score {score}   Misses {misses}   Time {remaining}s   Space pop cluster   Q menu"
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for y in 0..grid_h {
        for x in 0..grid_w {
            let color = grid[y as usize][x as usize];
            let role = match color {
                0 => Role::Success,
                1 => Role::Danger,
                2 => Role::Accent,
                3 => Role::Secondary,
                4 => Role::Highlight,
                _ => Role::Muted,
            };
            put(
                &mut buf,
                top + y as usize,
                left + (x * 2) as usize,
                "##",
                &theme,
                role,
                true,
            );
        }
    }
    put(
        &mut buf,
        top + cursor.1 as usize,
        left + (cursor.0 * 2) as usize,
        "<>",
        &theme,
        Role::Title,
        true,
    );
    flush(&buf);
}

fn game_bug_hunt(state: &mut AppState) {
    if !require_size(state, 22, 62, "Bug Hunt") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(52, 16, 132, 36);
        let mut cursor = (board_w / 2, board_h / 2);
        let mut bugs: Vec<(i32, i32)> = random_points(
            state,
            board_w,
            board_h,
            match state.difficulty_index {
                0 => 5,
                1 => 7,
                _ => 9,
            },
        )
        .into_iter()
        .collect();
        let mut score = 0u32;
        let mut misses = 0u32;
        let mut lives = state.starting_lives();
        let max_swarm = match state.difficulty_index {
            0 => 18,
            1 => 15,
            _ => 12,
        };
        let duration = match state.difficulty_index {
            0 => 60,
            1 => 45,
            _ => 35,
        };
        let mut end_at = Instant::now() + Duration::from_secs(duration);
        let mut last_move = Instant::now();
        let mut last_spawn = Instant::now();
        while lives > 0 && Instant::now() < end_at {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => cursor.1 = (cursor.1 - 1).max(1),
                    Key::Down | Key::Char('s') => cursor.1 = (cursor.1 + 1).min(board_h - 2),
                    Key::Left | Key::Char('a') => cursor.0 = (cursor.0 - 2).max(1),
                    Key::Right | Key::Char('d') => cursor.0 = (cursor.0 + 2).min(board_w - 2),
                    Key::Space | Key::Enter => {
                        if let Some(index) = bugs.iter().position(|bug| {
                            (bug.0 - cursor.0).abs() <= 1 && (bug.1 - cursor.1).abs() <= 1
                        }) {
                            bugs.remove(index);
                            score += 25;
                            play_sound(state, "score");
                        } else {
                            misses += 1;
                            score = score.saturating_sub(5);
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if let Some(paused) = pause_screen(state) {
                            end_at += paused;
                            last_move += paused;
                            last_spawn += paused;
                        } else {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_move.elapsed()
                >= Duration::from_millis((260.0 / state.difficulty().speed) as u64)
            {
                for bug in &mut bugs {
                    match state.rng.usize(5) {
                        0 => bug.0 = (bug.0 + 1).min(board_w - 2),
                        1 => bug.0 = (bug.0 - 1).max(1),
                        2 => bug.1 = (bug.1 + 1).min(board_h - 2),
                        3 => bug.1 = (bug.1 - 1).max(1),
                        _ => {}
                    }
                }
                last_move = Instant::now();
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((1800.0 / state.difficulty().speed) as u64)
            {
                bugs.push((
                    state.rng.range(2, board_w - 3),
                    state.rng.range(2, board_h - 3),
                ));
                last_spawn = Instant::now();
            }
            if bugs.len() > max_swarm {
                lives -= 1;
                bugs.truncate(max_swarm / 2);
                play_sound(state, "alert");
            }
            let remaining = end_at.saturating_duration_since(Instant::now()).as_secs();
            draw_bug_hunt(
                state, board_w, board_h, cursor, &bugs, score, misses, lives, remaining,
            );
            sleep_frame(frame, 45);
        }
        let final_score = score.saturating_sub(misses * 3);
        record_score(state, "Bug Hunt", final_score);
        if !wait_menu(
            state,
            "Bug Hunt",
            &[
                format!("Extermination report. Score: {final_score}"),
                "Shoot crawling bugs before the swarm overruns the board.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_bug_hunt(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    cursor: (i32, i32),
    bugs: &[(i32, i32)],
    score: u32,
    misses: u32,
    lives: i32,
    remaining: u64,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "BUG HUNT", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   Bugs {}   Misses {misses}   Time {remaining}s   Space shoot   Q menu", bugs.len()),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in bugs {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "b",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + cursor.1 as usize,
        left + cursor.0 as usize,
        "+",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_coin(state: &mut AppState) {
    if !require_size(state, 22, 58, "Coin Collector") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(48, 16, 118, 36);
        let mut player = (board_w / 2, board_h / 2);
        let mut coins = random_points(state, board_w, board_h, 7);
        let traps = random_points(
            state,
            board_w,
            board_h,
            match state.difficulty_index {
                0 => 5,
                1 => 7,
                _ => 10,
            },
        );
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        while lives > 0 && !coins.is_empty() {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Up | Key::Char('w') => player.1 = (player.1 - 1).max(1),
                    Key::Down | Key::Char('s') => player.1 = (player.1 + 1).min(board_h - 2),
                    Key::Left | Key::Char('a') => player.0 = (player.0 - 1).max(1),
                    Key::Right | Key::Char('d') => player.0 = (player.0 + 1).min(board_w - 2),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if coins.remove(&player) {
                score += 25;
                play_sound(state, "score");
            }
            if traps.contains(&player) {
                lives -= 1;
                player = (board_w / 2, board_h / 2);
                play_sound(state, "alert");
            }
            draw_grid_collect(
                state,
                "COIN COLLECTOR",
                board_w,
                board_h,
                player,
                &coins,
                &traps,
                "$",
                "x",
                lives,
                score,
            );
            sleep_frame(frame, 70);
        }
        record_score(state, "Coin Collector", score);
        if !wait_menu(
            state,
            "Coin Collector",
            &[
                format!("Run over. Score: {score}"),
                "Coins kept clinking.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn random_points(state: &mut AppState, w: i32, h: i32, count: usize) -> HashSet<(i32, i32)> {
    let mut points = HashSet::new();
    while points.len() < count {
        points.insert((state.rng.range(2, w - 3), state.rng.range(2, h - 3)));
    }
    points
}

fn draw_grid_collect(
    state: &AppState,
    title: &str,
    board_w: i32,
    board_h: i32,
    player: (i32, i32),
    good: &HashSet<(i32, i32)>,
    bad: &HashSet<(i32, i32)>,
    good_sprite: &str,
    bad_sprite: &str,
    lives: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, title, &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   WASD move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in good {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            good_sprite,
            &theme,
            Role::Success,
            true,
        );
    }
    for &(x, y) in bad {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            bad_sprite,
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize,
        "@",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_minefield(state: &mut AppState) {
    grid_exit_game(state, "Minefield", true, false);
}

fn game_maze(state: &mut AppState) {
    grid_exit_game(state, "Maze Runner", false, false);
}

fn game_circuit(state: &mut AppState) {
    grid_exit_game(state, "Circuit Trace", false, true);
}

#[derive(Clone, Copy)]
struct GridRules {
    mechanic: &'static str,
    mine_count: usize,
    ordered_count: i32,
    wanted_items: usize,
    lives: i32,
    bomb_defuse: bool,
    meltdown_steps: i32,
    moving_traps: bool,
    item_sprite: &'static str,
}

fn grid_rules(name: &str, difficulty_index: usize) -> GridRules {
    let difficulty = difficulty_index as i32;
    match name {
        "Minefield" => GridRules {
            mechanic: "hidden-mines-one-life-scanner",
            mine_count: match difficulty_index {
                0 => 14,
                1 => 22,
                _ => 32,
            },
            ordered_count: 0,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
        "Bomb Sweeper" => GridRules {
            mechanic: "adjacent-bomb-defuse-scanner",
            mine_count: match difficulty_index {
                0 => 18,
                1 => 26,
                _ => 36,
            },
            ordered_count: 0,
            wanted_items: 0,
            lives: 2,
            bomb_defuse: true,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
        "Maze Runner" => GridRules {
            mechanic: "plain-route-maze",
            mine_count: 0,
            ordered_count: 0,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
        "Circuit Trace" => GridRules {
            mechanic: "five-node-circuit-order",
            mine_count: 0,
            ordered_count: 5,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
        "Trap Runner" => GridRules {
            mechanic: "moving-visible-trap-grid",
            mine_count: 12 + difficulty_index * 4,
            ordered_count: 0,
            wanted_items: 0,
            lives: 3 + (2 - difficulty.min(2)),
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: true,
            item_sprite: "",
        },
        "Reactor Trace" => GridRules {
            mechanic: "reactor-countdown-node-route",
            mine_count: 0,
            ordered_count: 6,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 95 - difficulty * 15,
            moving_traps: false,
            item_sprite: "",
        },
        "Vault Escape" => GridRules {
            mechanic: "three-key-locked-vault",
            mine_count: 0,
            ordered_count: 0,
            wanted_items: 3,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "K",
        },
        "Ice Slide" => GridRules {
            mechanic: "ice-slide-stop-maze",
            mine_count: 0,
            ordered_count: 0,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
        "Signal Trace" => GridRules {
            mechanic: "seven-node-signal-long-route",
            mine_count: 0,
            ordered_count: 7,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
        "Crystal Cavern" => GridRules {
            mechanic: "many-crystal-cavern-sweep",
            mine_count: 0,
            ordered_count: 0,
            wanted_items: match difficulty_index {
                0 => 6,
                1 => 8,
                _ => 10,
            },
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "*",
        },
        _ => GridRules {
            mechanic: "fallback-grid-exit",
            mine_count: 0,
            ordered_count: 0,
            wanted_items: 0,
            lives: 1,
            bomb_defuse: false,
            meltdown_steps: 0,
            moving_traps: false,
            item_sprite: "",
        },
    }
}

fn grid_exit_game(state: &mut AppState, name: &str, mines_mode: bool, ordered_nodes: bool) {
    if !require_size(state, 22, 62, name) {
        return;
    }
    let rules = grid_rules(name, state.difficulty_index);
    let trap_mode = name == "Trap Runner";
    let vault_mode = name == "Vault Escape";
    let ice_mode = name == "Ice Slide";
    let crystal_mode = name == "Crystal Cavern";
    loop {
        let (w, h) = full_board(48, 16, 118, 36);
        let mut player = (1, h - 2);
        let goal = (w - 2, 1);
        let walls = if mines_mode {
            HashSet::new()
        } else {
            make_maze(state, w, h, player, goal)
        };
        let mut mines = if mines_mode {
            random_points(state, w, h, rules.mine_count)
        } else {
            HashSet::new()
        };
        let mut nodes = Vec::new();
        if ordered_nodes {
            for i in 0..rules.ordered_count {
                loop {
                    let point = (state.rng.range(4, w - 5), state.rng.range(3, h - 4));
                    if !walls.contains(&point) && point != player && point != goal {
                        nodes.push((point.0, point.1, i + 1));
                        break;
                    }
                }
            }
        }
        let mut items = HashSet::new();
        let wanted_items = rules.wanted_items;
        while items.len() < wanted_items {
            let point = (state.rng.range(2, w - 3), state.rng.range(2, h - 3));
            if !walls.contains(&point) && point != player && point != goal {
                items.insert(point);
            }
        }
        let item_sprite = rules.item_sprite;
        let mut next_node = 1;
        let mut steps = 0u32;
        let mut bonus = 0u32;
        let mut lives = rules.lives;
        let mut meltdown = rules.meltdown_steps;
        let mut status = format!("Find the exit. [{}]", rules.mechanic);
        let mut won = false;
        let mut lost = false;
        while !won && !lost {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                if is_pause(key) {
                    if pause_screen(state).is_none() {
                        return;
                    }
                    continue;
                }
                if rules.bomb_defuse && matches!(key, Key::Enter | Key::Space) {
                    let mut defused = None;
                    for &mine in &mines {
                        if (mine.0 - player.0).abs() <= 1 && (mine.1 - player.1).abs() <= 1 {
                            defused = Some(mine);
                            break;
                        }
                    }
                    if let Some(mine) = defused {
                        mines.remove(&mine);
                        bonus += 40;
                        status = "Bomb defused.".to_string();
                        play_sound(state, "score");
                    } else {
                        status = "No adjacent bomb to defuse.".to_string();
                    }
                    continue;
                }
                let delta = match key {
                    Key::Up | Key::Char('w') => (0, -1),
                    Key::Down | Key::Char('s') => (0, 1),
                    Key::Left | Key::Char('a') => (-1, 0),
                    Key::Right | Key::Char('d') => (1, 0),
                    _ if is_quit(key) => return,
                    _ => (0, 0),
                };
                if delta != (0, 0) {
                    if ice_mode {
                        let mut moved = false;
                        loop {
                            let next = (player.0 + delta.0, player.1 + delta.1);
                            if next.0 <= 0
                                || next.0 >= w - 1
                                || next.1 <= 0
                                || next.1 >= h - 1
                                || walls.contains(&next)
                            {
                                break;
                            }
                            player = next;
                            steps += 1;
                            moved = true;
                        }
                        if moved {
                            play_sound(state, "wall");
                        }
                    } else {
                        let next = (player.0 + delta.0, player.1 + delta.1);
                        if next.0 > 0
                            && next.0 < w - 1
                            && next.1 > 0
                            && next.1 < h - 1
                            && !walls.contains(&next)
                        {
                            player = next;
                            steps += 1;
                            if meltdown > 0 {
                                meltdown -= 1;
                            }
                        }
                    }
                }
            }
            if rules.moving_traps {
                let mut moved_traps = HashSet::new();
                for &trap in &mines {
                    let choices = [
                        (trap.0 + 1, trap.1),
                        (trap.0 - 1, trap.1),
                        (trap.0, trap.1 + 1),
                        (trap.0, trap.1 - 1),
                        trap,
                    ];
                    let next = choices[state.rng.usize(choices.len())];
                    if next.0 > 0 && next.0 < w - 1 && next.1 > 0 && next.1 < h - 1 && next != goal
                    {
                        moved_traps.insert(next);
                    } else {
                        moved_traps.insert(trap);
                    }
                }
                mines = moved_traps;
            }
            if items.remove(&player) {
                bonus += if vault_mode { 75 } else { 40 };
                play_sound(state, "score");
            }
            if mines.contains(&player) {
                if trap_mode {
                    lives -= 1;
                    if lives <= 0 {
                        lost = true;
                        status = "Caught by the traps.".to_string();
                    } else {
                        player = (1, h - 2);
                        status = format!("Trap hit. Lives {lives}.");
                    }
                } else if rules.bomb_defuse {
                    lives -= 1;
                    mines.remove(&player);
                    if lives <= 0 {
                        lost = true;
                        status = "Bomb blast ended the sweep.".to_string();
                    } else {
                        player = (1, h - 2);
                        status = format!("Bomb blast. Lives {lives}; keep sweeping.");
                    }
                } else {
                    lost = true;
                    status = "Boom. Hidden mine.".to_string();
                }
                play_sound(state, "alert");
            }
            if ordered_nodes {
                if let Some(pos) = nodes
                    .iter()
                    .position(|&(x, y, n)| (x, y) == player && n == next_node)
                {
                    nodes.remove(pos);
                    next_node += 1;
                    play_sound(state, "score");
                }
                if meltdown > 0 {
                    status = format!("Trace node {next_node}. Meltdown in {meltdown}.");
                } else if rules.meltdown_steps > 0 {
                    lost = true;
                    status = "Reactor meltdown. Route failed.".to_string();
                    play_sound(state, "alert");
                } else if name == "Signal Trace" {
                    status = format!("Trace long signal node {next_node}, then exit.");
                } else {
                    status = format!("Trace node {next_node}, then exit.");
                }
                if player == goal && next_node <= rules.ordered_count {
                    status = "Need all nodes first.".to_string();
                }
            } else if mines_mode && !lost {
                let nearby = mines
                    .iter()
                    .filter(|&&(x, y)| (x - player.0).abs() <= 1 && (y - player.1).abs() <= 1)
                    .count();
                if trap_mode {
                    status = format!("Lives {lives}. Moving traps nearby: {nearby}.");
                } else if rules.bomb_defuse {
                    status = format!("Scanner: {nearby} bomb(s) nearby. Space defuses adjacent.");
                } else {
                    status = format!("Scanner: {nearby} mine(s) nearby.");
                }
            } else if vault_mode {
                status = format!("Keys left: {}. Unlock the exit.", items.len());
                if player == goal && !items.is_empty() {
                    status = "Need every key first.".to_string();
                }
            } else if crystal_mode {
                status = format!("Crystals left: {}.", items.len());
                if player == goal && !items.is_empty() {
                    status = "Collect every crystal first.".to_string();
                }
            } else if ice_mode {
                status = "Sliding. Pick a direction and ride it out.".to_string();
            }
            if player == goal
                && (!ordered_nodes || next_node > rules.ordered_count)
                && (!vault_mode || items.is_empty())
                && (!crystal_mode || items.is_empty())
            {
                won = true;
                play_sound(state, "score");
            }
            draw_grid_exit(
                state,
                name,
                w,
                h,
                player,
                goal,
                &walls,
                &mines,
                &nodes,
                &items,
                item_sprite,
                ordered_nodes,
                steps,
                &status,
            );
            sleep_frame(frame, 70);
        }
        let score = if won {
            1000u32.saturating_sub(steps * 3) + bonus
        } else {
            steps + bonus
        };
        record_score(state, name, score);
        if !wait_menu(state, name, &[status, format!("Score: {score}")], true) {
            return;
        }
    }
}

fn make_maze(
    state: &mut AppState,
    w: i32,
    h: i32,
    start: (i32, i32),
    goal: (i32, i32),
) -> HashSet<(i32, i32)> {
    for _ in 0..200 {
        let mut walls = HashSet::new();
        for y in 1..h - 1 {
            for x in 1..w - 1 {
                if (x, y) != start
                    && (x, y) != goal
                    && state.rng.chance(
                        match state.difficulty_index {
                            0 => 12,
                            1 => 17,
                            _ => 22,
                        },
                        100,
                    )
                {
                    walls.insert((x, y));
                }
            }
        }
        if path_exists(w, h, start, goal, &walls) {
            return walls;
        }
    }
    HashSet::new()
}

fn path_exists(
    w: i32,
    h: i32,
    start: (i32, i32),
    goal: (i32, i32),
    walls: &HashSet<(i32, i32)>,
) -> bool {
    let mut queue = VecDeque::new();
    let mut seen = HashSet::new();
    queue.push_back(start);
    seen.insert(start);
    while let Some(p) = queue.pop_front() {
        if p == goal {
            return true;
        }
        for n in [
            (p.0 + 1, p.1),
            (p.0 - 1, p.1),
            (p.0, p.1 + 1),
            (p.0, p.1 - 1),
        ] {
            if n.0 > 0
                && n.0 < w - 1
                && n.1 > 0
                && n.1 < h - 1
                && !walls.contains(&n)
                && seen.insert(n)
            {
                queue.push_back(n);
            }
        }
    }
    false
}

fn draw_grid_exit(
    state: &AppState,
    name: &str,
    w: i32,
    h: i32,
    player: (i32, i32),
    goal: (i32, i32),
    walls: &HashSet<(i32, i32)>,
    mines: &HashSet<(i32, i32)>,
    nodes: &[(i32, i32, i32)],
    items: &HashSet<(i32, i32)>,
    item_sprite: &str,
    ordered_nodes: bool,
    steps: u32,
    status: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - h as usize / 2 + 1;
    let left = cols / 2 - w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        &name.to_ascii_uppercase(),
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Steps {steps}   {status}   WASD move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        h as usize + 2,
        w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in walls {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "#",
            &theme,
            Role::Muted,
            false,
        );
    }
    if name == "Trap Runner" {
        for &(x, y) in mines {
            put(
                &mut buf,
                top + y as usize,
                left + x as usize,
                "^",
                &theme,
                Role::Danger,
                true,
            );
        }
    }
    if !item_sprite.is_empty() {
        for &(x, y) in items {
            put(
                &mut buf,
                top + y as usize,
                left + x as usize,
                item_sprite,
                &theme,
                Role::Success,
                true,
            );
        }
    }
    if ordered_nodes {
        for &(x, y, n) in nodes {
            put(
                &mut buf,
                top + y as usize,
                left + x as usize,
                &n.to_string(),
                &theme,
                Role::Success,
                true,
            );
        }
    }
    if mines.is_empty() {
        put(
            &mut buf,
            top + goal.1 as usize,
            left + goal.0 as usize,
            "G",
            &theme,
            Role::Success,
            true,
        );
    } else {
        put(
            &mut buf,
            top + goal.1 as usize,
            left + goal.0 as usize,
            "E",
            &theme,
            Role::Success,
            true,
        );
    }
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize,
        "@",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_byte_blaster(state: &mut AppState) {
    if !require_size(state, 22, 62, "Byte Blaster") {
        return;
    }
    let keys = ['a', 's', 'd', 'f', 'j', 'k', 'l', 'w', 'e', 'r', 'u', 'i'];
    loop {
        let (board_w, board_h) = full_board(56, 17, 132, 38);
        let mut bytes: Vec<(i32, f64, char)> = Vec::new();
        let mut score = 0u32;
        let mut streak = 0u32;
        let mut lives = state.starting_lives();
        let mut last_spawn = Instant::now();
        while lives > 0 {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Char(ch) if ch != 'q' => {
                        let mut hit_index = None;
                        let mut lowest_y = -1.0;
                        for (index, byte) in bytes.iter().enumerate() {
                            if byte.2 == ch && byte.1 > lowest_y {
                                hit_index = Some(index);
                                lowest_y = byte.1;
                            }
                        }
                        if let Some(index) = hit_index {
                            bytes.remove(index);
                            score += 20 + streak.min(12) * 2;
                            streak += 1;
                            play_sound(state, "score");
                        } else {
                            streak = 0;
                            score = score.saturating_sub(2);
                            play_sound(state, "wall");
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((520.0 / state.difficulty().speed) as u64)
            {
                let ch = keys[state.rng.usize(keys.len())];
                bytes.push((state.rng.range(2, board_w - 3), 1.0, ch));
                if state.difficulty_index >= 2 && state.rng.chance(1, 4) {
                    let ch = keys[state.rng.usize(keys.len())];
                    bytes.push((state.rng.range(2, board_w - 3), 1.0, ch));
                }
                last_spawn = Instant::now();
            }
            for byte in &mut bytes {
                byte.1 += 0.20 * state.difficulty().speed;
            }
            let mut kept = Vec::new();
            for byte in bytes.into_iter() {
                if byte.1.round() as i32 >= board_h - 1 {
                    lives -= 1;
                    streak = 0;
                    play_sound(state, "alert");
                } else {
                    kept.push(byte);
                }
            }
            bytes = kept;
            draw_byte_blaster(state, board_w, board_h, &bytes, lives, score, streak);
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, "Byte Blaster", score);
        if !wait_menu(
            state,
            "Byte Blaster",
            &[
                format!("System flooded. Score: {score}"),
                "Type matching letters before they hit the bottom.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_byte_blaster(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    bytes: &[(i32, f64, char)],
    lives: i32,
    score: u32,
    streak: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "BYTE BLASTER", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!(
            "Score {score}   Lives {lives}   Streak {streak}   Type matching letters   Q menu"
        ),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    put(
        &mut buf,
        top + board_h as usize - 1,
        left,
        &"=".repeat(board_w as usize),
        &theme,
        Role::Danger,
        false,
    );
    for &(x, y, ch) in bytes {
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + x as usize,
            &ch.to_string(),
            &theme,
            if y > (board_h - 5) as f64 {
                Role::Danger
            } else {
                Role::Success
            },
            true,
        );
    }
    flush(&buf);
}

fn game_dungeon(state: &mut AppState) {
    if !require_size(state, 22, 62, "Dungeon Crawl") {
        return;
    }
    loop {
        let (w, h) = full_board(48, 16, 118, 36);
        let mut player = (2, h - 2);
        let exit = (w - 3, 1);
        let walls = make_maze(state, w, h, player, exit);
        let mut treasure = random_points(state, w, h, 6);
        let mut enemies: Vec<(i32, i32)> = random_points(
            state,
            w,
            h,
            match state.difficulty_index {
                0 => 3,
                1 => 4,
                _ => 5,
            },
        )
        .into_iter()
        .collect();
        let mut score = 0u32;
        let mut alive = true;
        let mut won = false;
        while alive && !won {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                let mut next = player;
                match key {
                    Key::Up | Key::Char('w') => next.1 -= 1,
                    Key::Down | Key::Char('s') => next.1 += 1,
                    Key::Left | Key::Char('a') => next.0 -= 1,
                    Key::Right | Key::Char('d') => next.0 += 1,
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
                if next.0 > 0
                    && next.0 < w - 1
                    && next.1 > 0
                    && next.1 < h - 1
                    && !walls.contains(&next)
                {
                    player = next;
                }
            }
            if treasure.remove(&player) {
                score += 100;
                play_sound(state, "score");
            }
            for enemy in &mut enemies {
                let choices = [
                    (enemy.0 + 1, enemy.1),
                    (enemy.0 - 1, enemy.1),
                    (enemy.0, enemy.1 + 1),
                    (enemy.0, enemy.1 - 1),
                ];
                let next = choices[state.rng.usize(choices.len())];
                if next.0 > 0
                    && next.0 < w - 1
                    && next.1 > 0
                    && next.1 < h - 1
                    && !walls.contains(&next)
                {
                    *enemy = next;
                }
            }
            if enemies.contains(&player) {
                alive = false;
                play_sound(state, "alert");
            }
            if player == exit {
                won = true;
                score += 250;
            }
            draw_dungeon(
                state, w, h, player, exit, &walls, &treasure, &enemies, score,
            );
            sleep_frame(frame, 120);
        }
        record_score(state, "Dungeon Crawl", score);
        let result = if won {
            "You reached the stairs."
        } else {
            "A dungeon enemy caught you."
        };
        if !wait_menu(
            state,
            "Dungeon Crawl",
            &[result.to_string(), format!("Score: {score}")],
            true,
        ) {
            return;
        }
    }
}

fn draw_dungeon(
    state: &AppState,
    w: i32,
    h: i32,
    player: (i32, i32),
    exit: (i32, i32),
    walls: &HashSet<(i32, i32)>,
    treasure: &HashSet<(i32, i32)>,
    enemies: &[(i32, i32)],
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - h as usize / 2 + 1;
    let left = cols / 2 - w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        0,
        "DUNGEON CRAWL",
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        1,
        &format!("Score {score}   Grab $   Reach >   WASD move   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        h as usize + 2,
        w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in walls {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "#",
            &theme,
            Role::Muted,
            false,
        );
    }
    for &(x, y) in treasure {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "$",
            &theme,
            Role::Success,
            true,
        );
    }
    for &(x, y) in enemies {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "e",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + exit.1 as usize,
        left + exit.0 as usize,
        ">",
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + player.1 as usize,
        left + player.0 as usize,
        "@",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_laser(state: &mut AppState) {
    if !require_size(state, 22, 62, "Laser Drill") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(52, 17, 132, 38);
        let mut player_x = board_w / 2;
        let mut shots: Vec<(i32, i32)> = Vec::new();
        let mut blocks: Vec<(i32, f64)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut last_spawn = Instant::now();
        while lives > 0 {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => player_x = (player_x - 2).max(2),
                    Key::Right | Key::Char('d') => player_x = (player_x + 2).min(board_w - 3),
                    Key::Space => {
                        shots.push((player_x, board_h - 3));
                        play_sound(state, "paddle");
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((440.0 / state.difficulty().speed) as u64)
            {
                blocks.push((state.rng.range(2, board_w - 3), 1.0));
                last_spawn = Instant::now();
            }
            for shot in &mut shots {
                shot.1 -= 1;
            }
            for block in &mut blocks {
                block.1 += 0.23 * state.difficulty().speed;
            }
            let mut hit_blocks = HashSet::new();
            let mut hit_shots = HashSet::new();
            for (si, shot) in shots.iter().enumerate() {
                for (bi, block) in blocks.iter().enumerate() {
                    if (shot.0 - block.0).abs() <= 1 && (shot.1 - block.1.round() as i32).abs() <= 1
                    {
                        hit_shots.insert(si);
                        hit_blocks.insert(bi);
                    }
                }
            }
            if !hit_blocks.is_empty() {
                score += hit_blocks.len() as u32 * 20;
                play_sound(state, "score");
            }
            shots = shots
                .into_iter()
                .enumerate()
                .filter_map(|(i, shot)| (shot.1 > 0 && !hit_shots.contains(&i)).then_some(shot))
                .collect();
            let mut kept = Vec::new();
            for (i, block) in blocks.into_iter().enumerate() {
                if hit_blocks.contains(&i) {
                    continue;
                }
                if block.1 >= board_h as f64 - 2.0 {
                    lives -= 1;
                    play_sound(state, "alert");
                } else {
                    kept.push(block);
                }
            }
            blocks = kept;
            draw_laser(
                state, board_w, board_h, player_x, &shots, &blocks, lives, score,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, "Laser Drill", score);
        if !wait_menu(
            state,
            "Laser Drill",
            &[
                format!("Drill overheated. Score: {score}"),
                "Blocks reached the floor.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_laser(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    player_x: i32,
    shots: &[(i32, i32)],
    blocks: &[(i32, f64)],
    lives: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "LASER DRILL", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   A/D move   Space fire   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    for &(x, y) in shots {
        put(
            &mut buf,
            top + y as usize,
            left + x as usize,
            "|",
            &theme,
            Role::Success,
            true,
        );
    }
    for &(x, y) in blocks {
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + x as usize - 1,
            "[]",
            &theme,
            Role::Danger,
            true,
        );
    }
    put(
        &mut buf,
        top + board_h as usize - 2,
        left + player_x as usize - 1,
        "/A\\",
        &theme,
        Role::Secondary,
        true,
    );
    flush(&buf);
}

fn game_simon(state: &mut AppState) {
    let keys = [('w', "UP"), ('a', "LEFT"), ('s', "DOWN"), ('d', "RIGHT")];
    loop {
        let mut sequence: Vec<usize> = Vec::new();
        let mut score = 0u32;
        let mut round_no = 0u32;
        loop {
            sequence.push(state.rng.usize(keys.len()));
            round_no += 1;
            for &index in &sequence {
                draw_prompt(
                    state,
                    "SIMON SAYS",
                    &format!("Round {round_no}"),
                    keys[index].1,
                    "Watch the sequence.",
                );
                thread::sleep(Duration::from_millis(match state.difficulty_index {
                    0 => 760,
                    1 => 560,
                    _ => 390,
                }));
                draw_prompt(state, "SIMON SAYS", "", "", "");
                thread::sleep(Duration::from_millis(140));
            }
            draw_prompt(
                state,
                "SIMON SAYS",
                "Repeat it with W/A/S/D. Q quits.",
                "",
                "",
            );
            for &index in &sequence {
                let expected = keys[index].0;
                let got = loop {
                    let Some(key) = wait_for_key() else {
                        record_score(state, "Simon Says", score);
                        return;
                    };
                    if is_pause(key) {
                        if pause_screen(state).is_none() {
                            record_score(state, "Simon Says", score);
                            return;
                        }
                        draw_prompt(
                            state,
                            "SIMON SAYS",
                            "Repeat it with W/A/S/D. Q quits.",
                            "",
                            "",
                        );
                        continue;
                    }
                    break key;
                };
                if is_quit(got) {
                    record_score(state, "Simon Says", score);
                    return;
                }
                if got != Key::Char(expected) {
                    record_score(state, "Simon Says", score);
                    if !wait_menu(
                        state,
                        "Simon Says",
                        &[
                            format!("Sequence broke. Score: {score}"),
                            format!("Round: {round_no}"),
                        ],
                        true,
                    ) {
                        return;
                    }
                    break;
                }
            }
            score += sequence.len() as u32 * 10;
            if sequence.len() > 40 {
                record_score(state, "Simon Says", score);
                if !wait_menu(
                    state,
                    "Simon Says",
                    &[format!("Huge memory. Score: {score}")],
                    true,
                ) {
                    return;
                }
                break;
            }
        }
    }
}

fn draw_prompt(state: &AppState, title: &str, line1: &str, line2: &str, line3: &str) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        rows / 2 - 5,
        title,
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 - 2,
        line1,
        &theme,
        Role::Accent,
        false,
        cols,
    );
    center(
        &mut buf,
        rows / 2,
        line2,
        &theme,
        Role::Highlight,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 3,
        line3,
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn wait_for_key() -> Option<Key> {
    loop {
        if let Some(key) = read_key() {
            return Some(key);
        }
        thread::sleep(Duration::from_millis(15));
    }
}

fn game_reaction(state: &mut AppState) {
    loop {
        let keys = ['w', 'a', 's', 'd', 'j', 'k', 'l'];
        let rounds = match state.difficulty_index {
            0 => 10,
            1 => 12,
            _ => 14,
        };
        let mut score = 0u32;
        let mut misses = 0u32;
        for round in 1..=rounds {
            draw_prompt(
                state,
                "REACTION TEST",
                &format!("Round {round}/{rounds}"),
                "Get ready...",
                "",
            );
            thread::sleep(Duration::from_millis(state.rng.range(
                350,
                match state.difficulty_index {
                    0 => 1400,
                    1 => 1000,
                    _ => 760,
                },
            ) as u64));
            let prompt = keys[state.rng.usize(keys.len())];
            let mut start = Instant::now();
            draw_prompt(
                state,
                "REACTION TEST",
                "",
                &format!("PRESS {}", prompt.to_ascii_uppercase()),
                "Q quits.",
            );
            loop {
                let Some(key) = wait_for_key() else {
                    break;
                };
                if is_pause(key) {
                    if let Some(paused) = pause_screen(state) {
                        start += paused;
                        draw_prompt(
                            state,
                            "REACTION TEST",
                            "",
                            &format!("PRESS {}", prompt.to_ascii_uppercase()),
                            "Q quits.",
                        );
                        continue;
                    }
                    return;
                }
                if is_quit(key) {
                    return;
                }
                let elapsed = start.elapsed().as_millis() as u32;
                if key == Key::Char(prompt) {
                    score += 120u32.saturating_sub(elapsed / 8).max(5);
                    play_sound(state, "score");
                } else {
                    misses += 1;
                }
                break;
            }
        }
        let final_score = score.saturating_sub(misses * 25);
        record_score(state, "Reaction Test", final_score);
        if !wait_menu(
            state,
            "Reaction Test",
            &[format!("Score: {final_score}"), format!("Misses: {misses}")],
            true,
        ) {
            return;
        }
    }
}

fn game_memory(state: &mut AppState) {
    if !require_size(state, 22, 58, "Memory Match") {
        return;
    }
    loop {
        let mut values: Vec<char> = "AABBCCDDEEFFGGHH".chars().collect();
        for i in 0..values.len() {
            let j = state.rng.usize(values.len());
            values.swap(i, j);
        }
        let mut revealed = vec![false; 16];
        let mut cursor = 0usize;
        let mut first: Option<usize> = None;
        let mut moves = 0u32;
        let mut matched = 0usize;
        while matched < 16 {
            draw_memory(state, &values, &revealed, cursor, first, moves);
            if let Some(key) = wait_for_key() {
                match key {
                    Key::Up | Key::Char('w') if cursor >= 4 => cursor -= 4,
                    Key::Down | Key::Char('s') if cursor < 12 => cursor += 4,
                    Key::Left | Key::Char('a') if cursor % 4 > 0 => cursor -= 1,
                    Key::Right | Key::Char('d') if cursor % 4 < 3 => cursor += 1,
                    Key::Enter | Key::Space if !revealed[cursor] => {
                        if let Some(prev) = first {
                            moves += 1;
                            if values[prev] == values[cursor] {
                                revealed[prev] = true;
                                revealed[cursor] = true;
                                matched += 2;
                                play_sound(state, "score");
                            } else {
                                draw_memory_peek(state, &values, &revealed, cursor, prev, moves);
                                thread::sleep(Duration::from_millis(650));
                            }
                            first = None;
                        } else {
                            first = Some(cursor);
                        }
                    }
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
        }
        let score = 1000u32.saturating_sub(moves * 25);
        record_score(state, "Memory Match", score);
        if !wait_menu(
            state,
            "Memory Match",
            &[
                format!("All matched. Score: {score}"),
                format!("Moves: {moves}"),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_memory(
    state: &AppState,
    values: &[char],
    revealed: &[bool],
    cursor: usize,
    first: Option<usize>,
    moves: u32,
) {
    draw_memory_inner(state, values, revealed, cursor, first, None, moves);
}

fn draw_memory_peek(
    state: &AppState,
    values: &[char],
    revealed: &[bool],
    cursor: usize,
    prev: usize,
    moves: u32,
) {
    draw_memory_inner(
        state,
        values,
        revealed,
        cursor,
        Some(prev),
        Some(cursor),
        moves,
    );
}

fn draw_memory_inner(
    state: &AppState,
    values: &[char],
    revealed: &[bool],
    cursor: usize,
    first: Option<usize>,
    second: Option<usize>,
    moves: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - 5;
    let left = cols / 2 - 10;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 1, "MEMORY MATCH", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        2,
        &format!("Moves {moves}   WASD move   Enter flip   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    for i in 0..16 {
        let row = top + (i / 4) * 2;
        let col = left + (i % 4) * 5;
        let shown = revealed[i] || first == Some(i) || second == Some(i);
        let text = if shown {
            format!("[{}]", values[i])
        } else {
            "[?]".to_string()
        };
        if i == cursor {
            put_inv(&mut buf, row, col, &text, &theme, Role::Highlight);
        } else {
            put(
                &mut buf,
                row,
                col,
                &text,
                &theme,
                if shown { Role::Success } else { Role::Muted },
                shown,
            );
        }
    }
    flush(&buf);
}

fn game_number(state: &mut AppState) {
    loop {
        let duration = match state.difficulty_index {
            0 => 45,
            1 => 35,
            _ => 25,
        };
        let mut end_at = Instant::now() + Duration::from_secs(duration);
        let mut score = 0u32;
        let mut misses = 0u32;
        while Instant::now() < end_at {
            let a = state
                .rng
                .range(2, if state.difficulty_index >= 2 { 30 } else { 18 });
            let b = state
                .rng
                .range(2, if state.difficulty_index >= 2 { 20 } else { 12 });
            let op = state.rng.usize(3);
            let (prompt, answer) = match op {
                0 => (format!("{a} + {b} ="), a + b),
                1 => (
                    format!("{} - {} =", a.max(b), a.min(b)),
                    a.max(b) - a.min(b),
                ),
                _ => (format!("{a} x {b} ="), a * b),
            };
            let mut typed = String::new();
            loop {
                draw_number(
                    state,
                    score,
                    misses,
                    end_at.saturating_duration_since(Instant::now()).as_secs(),
                    &prompt,
                    &typed,
                );
                if Instant::now() >= end_at {
                    break;
                }
                if let Some(key) = read_key() {
                    match key {
                        Key::Char(c) if c.is_ascii_digit() => typed.push(c),
                        Key::Backspace => {
                            typed.pop();
                        }
                        Key::Enter => {
                            if typed.parse::<i32>().ok() == Some(answer) {
                                score += 40;
                                play_sound(state, "score");
                            } else {
                                misses += 1;
                            }
                            break;
                        }
                        _ if is_pause(key) => {
                            if let Some(paused) = pause_screen(state) {
                                end_at += paused;
                            } else {
                                return;
                            }
                        }
                        _ if is_quit(key) => return,
                        _ => {}
                    }
                }
                thread::sleep(Duration::from_millis(20));
            }
        }
        let final_score = score.saturating_sub(misses * 10);
        record_score(state, "Number Crunch", final_score);
        if !wait_menu(
            state,
            "Number Crunch",
            &[format!("Score: {final_score}"), format!("Misses: {misses}")],
            true,
        ) {
            return;
        }
    }
}

fn draw_number(
    state: &AppState,
    score: u32,
    misses: u32,
    remaining: u64,
    prompt: &str,
    typed: &str,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(
        &mut buf,
        rows / 2 - 5,
        "NUMBER CRUNCH",
        &theme,
        Role::Title,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 - 2,
        &format!("Score {score}   Misses {misses}   Time {remaining}s"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    center(
        &mut buf,
        rows / 2,
        prompt,
        &theme,
        Role::Highlight,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 2,
        &format!("> {typed}"),
        &theme,
        Role::Success,
        true,
        cols,
    );
    center(
        &mut buf,
        rows / 2 + 5,
        "Type answer, Enter submits, Q quits.",
        &theme,
        Role::Muted,
        false,
        cols,
    );
    flush(&buf);
}

fn game_orbit(state: &mut AppState) {
    if !require_size(state, 22, 62, "Orbit Guard") {
        return;
    }
    loop {
        let (board_w, board_h) = full_board(52, 17, 132, 38);
        let center_p = (board_w / 2, board_h / 2);
        let guard_positions = [
            (0, -5),
            (4, -3),
            (5, 0),
            (4, 3),
            (0, 5),
            (-4, 3),
            (-5, 0),
            (-4, -3),
        ];
        let mut guard = 0usize;
        let mut sparks: Vec<(f64, f64, f64, f64)> = Vec::new();
        let mut lives = state.starting_lives();
        let mut score = 0u32;
        let mut last_spawn = Instant::now();
        while lives > 0 {
            let frame = Instant::now();
            while let Some(key) = read_key() {
                match key {
                    Key::Left | Key::Char('a') => {
                        guard = (guard + guard_positions.len() - 1) % guard_positions.len()
                    }
                    Key::Right | Key::Char('d') => guard = (guard + 1) % guard_positions.len(),
                    _ if is_pause(key) => {
                        if pause_screen(state).is_none() {
                            return;
                        }
                    }
                    _ if is_quit(key) => return,
                    _ => {}
                }
            }
            if last_spawn.elapsed()
                >= Duration::from_millis((520.0 / state.difficulty().speed) as u64)
            {
                let side = state.rng.usize(4);
                let (sx, sy) = match side {
                    0 => (state.rng.range(1, board_w - 2) as f64, 1.0),
                    1 => (state.rng.range(1, board_w - 2) as f64, board_h as f64 - 2.0),
                    2 => (1.0, state.rng.range(1, board_h - 2) as f64),
                    _ => (board_w as f64 - 2.0, state.rng.range(1, board_h - 2) as f64),
                };
                let dx = center_p.0 as f64 - sx;
                let dy = center_p.1 as f64 - sy;
                let len = (dx * dx + dy * dy).sqrt();
                sparks.push((
                    sx,
                    sy,
                    dx / len * 0.35 * state.difficulty().speed,
                    dy / len * 0.35 * state.difficulty().speed,
                ));
                last_spawn = Instant::now();
            }
            for spark in &mut sparks {
                spark.0 += spark.2;
                spark.1 += spark.3;
            }
            let guard_pos = (
                center_p.0 + guard_positions[guard].0,
                center_p.1 + guard_positions[guard].1,
            );
            let mut kept = Vec::new();
            for spark in sparks.into_iter() {
                let sp = (spark.0.round() as i32, spark.1.round() as i32);
                if (sp.0 - guard_pos.0).abs() <= 1 && (sp.1 - guard_pos.1).abs() <= 1 {
                    score += 20;
                    play_sound(state, "score");
                } else if (sp.0 - center_p.0).abs() <= 1 && (sp.1 - center_p.1).abs() <= 1 {
                    lives -= 1;
                    play_sound(state, "alert");
                } else {
                    kept.push(spark);
                }
            }
            sparks = kept;
            draw_orbit(
                state, board_w, board_h, center_p, guard_pos, &sparks, lives, score,
            );
            sleep_frame(frame, state.difficulty().tick_ms);
        }
        record_score(state, "Orbit Guard", score);
        if !wait_menu(
            state,
            "Orbit Guard",
            &[
                format!("Core breached. Score: {score}"),
                "Rotate with A/D or arrows.".to_string(),
            ],
            true,
        ) {
            return;
        }
    }
}

fn draw_orbit(
    state: &AppState,
    board_w: i32,
    board_h: i32,
    center_p: (i32, i32),
    guard_pos: (i32, i32),
    sparks: &[(f64, f64, f64, f64)],
    lives: i32,
    score: u32,
) {
    let (rows, cols) = terminal_size();
    let theme = state.theme().clone();
    let top = rows / 2 - board_h as usize / 2 + 1;
    let left = cols / 2 - board_w as usize / 2;
    let mut buf = String::new();
    clear_buf(&mut buf, &theme);
    center(&mut buf, 0, "ORBIT GUARD", &theme, Role::Title, true, cols);
    center(
        &mut buf,
        1,
        &format!("Score {score}   Lives {lives}   A/D rotate   Q menu"),
        &theme,
        Role::Accent,
        false,
        cols,
    );
    draw_box(
        &mut buf,
        top - 1,
        left - 1,
        board_h as usize + 2,
        board_w as usize + 2,
        "",
        &theme,
        Role::Accent,
        state.glyphs(),
    );
    put(
        &mut buf,
        top + center_p.1 as usize,
        left + center_p.0 as usize,
        "O",
        &theme,
        Role::Success,
        true,
    );
    put(
        &mut buf,
        top + guard_pos.1 as usize,
        left + guard_pos.0 as usize,
        "#",
        &theme,
        Role::Secondary,
        true,
    );
    for &(x, y, _, _) in sparks {
        put(
            &mut buf,
            top + y.round().max(0.0) as usize,
            left + x.round().max(0.0) as usize,
            "*",
            &theme,
            Role::Danger,
            true,
        );
    }
    flush(&buf);
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn game_names_are_unique() {
        let mut names = HashSet::new();
        for game in GAMES {
            assert!(
                names.insert(game.name),
                "duplicate game name: {}",
                game.name
            );
        }
    }

    #[test]
    fn micro_indices_are_valid_and_complete() {
        let mut referenced = HashSet::new();
        for game in GAMES {
            if let GameKind::Micro(index) = &game.kind {
                assert!(
                    *index < MICRO_GAMES.len(),
                    "micro game index {} out of range for {}",
                    index,
                    game.name
                );
                referenced.insert(*index);
            }
        }
        assert_eq!(
            referenced.len(),
            MICRO_GAMES.len(),
            "every micro game spec should be referenced exactly once"
        );
    }

    #[test]
    fn micro_games_have_distinct_dispatch_modes() {
        let mut labels = HashSet::new();
        for spec in MICRO_GAMES {
            let label = micro_mode_label(spec.mode);
            assert!(labels.insert(label.clone()), "repeated micro mode: {label}");
        }
    }

    #[test]
    fn visible_games_have_distinct_mechanic_signatures() {
        let mut labels = HashSet::new();
        for game in GAMES {
            let label = game_mechanic_label(game);
            assert!(
                labels.insert(label.clone()),
                "repeated game mechanic signature: {} ({})",
                label,
                game.name
            );
        }
    }

    #[test]
    fn master_difficulty_and_pong_options_exist() {
        let master = DIFFICULTIES
            .last()
            .expect("difficulty list should not be empty");
        assert_eq!(master.name, "Master");
        assert!(master.speed > DIFFICULTIES[2].speed);
        assert!(master.tick_ms < DIFFICULTIES[2].tick_ms);
        assert!(master.lives < DIFFICULTIES[2].lives);
        assert_eq!(PONG_ASSIST_NAMES, ["Off", "Light", "Strong"]);
        assert_eq!(PONG_SPEED_NAMES, ["Calm", "Classic", "Fast"]);
    }

    #[test]
    fn sound_cooldowns_prevent_spam() {
        assert!(sound_gap_ms("alert") > sound_gap_ms("score"));
        assert!(sound_gap_ms("score") > sound_gap_ms("click"));
        assert!(sound_gap_ms("paddle") >= 90);
    }

    #[test]
    fn chess_setup_and_legal_moves_work() {
        let board = chess_initial_board();
        assert_eq!(board[chess_idx(4, 7)], 'K');
        assert_eq!(board[chess_idx(4, 0)], 'k');
        assert!(chess_legal_move(
            &board,
            chess_idx(4, 6),
            chess_idx(4, 4),
            true
        ));
        assert!(!chess_legal_move(
            &board,
            chess_idx(4, 6),
            chess_idx(4, 3),
            true
        ));
        assert!(chess_legal_move(
            &board,
            chess_idx(6, 7),
            chess_idx(5, 5),
            true
        ));
        let moved = chess_make_move(board, chess_idx(4, 6), chess_idx(4, 4));
        assert_eq!(moved[chess_idx(4, 4)], 'P');
        assert!(!chess_in_check(&moved, true));
        assert!(!chess_all_legal_moves(&moved, false).is_empty());
    }

    #[test]
    fn checkers_setup_and_capture_rules_work() {
        let board = checkers_initial_board();
        assert_eq!(checkers_count(&board, true), 12);
        assert_eq!(checkers_count(&board, false), 12);
        assert!(!checkers_legal_moves(&board, true).is_empty());
        let mut capture_board = ['.'; 64];
        capture_board[checkers_idx(2, 5)] = 'w';
        capture_board[checkers_idx(3, 4)] = 'b';
        let moves = checkers_legal_moves(&capture_board, true);
        assert_eq!(
            moves,
            vec![(
                checkers_idx(2, 5),
                checkers_idx(4, 3),
                Some(checkers_idx(3, 4))
            )]
        );
        checkers_apply_move(
            &mut capture_board,
            checkers_idx(2, 5),
            checkers_idx(4, 3),
            Some(checkers_idx(3, 4)),
        );
        assert_eq!(capture_board[checkers_idx(4, 3)], 'w');
        assert_eq!(capture_board[checkers_idx(3, 4)], '.');
    }

    #[test]
    fn tron_turns_reject_reverse_and_find_safe_paths() {
        assert_eq!(
            tron_turn_dir(TronDir::Right, Key::Char('a')) as u8,
            TronDir::Right as u8
        );
        assert_eq!(
            tron_turn_dir(TronDir::Right, Key::Char('w')) as u8,
            TronDir::Up as u8
        );
        let mut occupied = HashSet::new();
        occupied.insert((5, 4));
        let safe = tron_safe_dirs((5, 5), TronDir::Right, 10, 10, &occupied);
        assert!(!safe.contains(&TronDir::Left));
        assert!(!safe.contains(&TronDir::Up));
        assert!(safe.contains(&TronDir::Right));
        assert!(safe.contains(&TronDir::Down));
    }

    #[test]
    fn shared_engine_rule_profiles_are_distinct() {
        assert_distinct(
            "falling profile",
            &[
                falling_rules("Meteor Dodge", 1).mechanic,
                falling_rules("Star Catcher", 1).mechanic,
                falling_rules("Block Drop", 1).mechanic,
                falling_rules("Comet Catcher", 1).mechanic,
                falling_rules("Cargo Catch", 1).mechanic,
                falling_rules("Gem Rush", 1).mechanic,
                falling_rules("Pearl Diver", 1).mechanic,
                falling_rules("Data Storm", 1).mechanic,
                falling_rules("Rain Runner", 1).mechanic,
            ],
        );
        assert_distinct(
            "scroll profile",
            &[
                scroll_rules("Asteroid Belt").mechanic,
                scroll_rules("River Raid").mechanic,
                scroll_rules("Neon Drift").mechanic,
                scroll_rules("Drone Dodge").mechanic,
                scroll_rules("Solar Sailer").mechanic,
                scroll_rules("Fuel Run").mechanic,
                scroll_rules("Spark Chase").mechanic,
                scroll_rules("Orbital Courier").mechanic,
                scroll_rules("Storm Surge").mechanic,
            ],
        );
        assert_distinct(
            "grid profile",
            &[
                grid_rules("Minefield", 1).mechanic,
                grid_rules("Bomb Sweeper", 1).mechanic,
                grid_rules("Maze Runner", 1).mechanic,
                grid_rules("Circuit Trace", 1).mechanic,
                grid_rules("Trap Runner", 1).mechanic,
                grid_rules("Reactor Trace", 1).mechanic,
                grid_rules("Vault Escape", 1).mechanic,
                grid_rules("Ice Slide", 1).mechanic,
                grid_rules("Signal Trace", 1).mechanic,
                grid_rules("Crystal Cavern", 1).mechanic,
            ],
        );
        assert_distinct(
            "micro lane profile",
            &[
                lane_rules(LaneKind::Rune).mechanic,
                lane_rules(LaneKind::Sea).mechanic,
                lane_rules(LaneKind::AirHockey).mechanic,
                lane_rules(LaneKind::Hockey).mechanic,
                lane_rules(LaneKind::Ski).mechanic,
                lane_rules(LaneKind::Snowboard).mechanic,
                lane_rules(LaneKind::Bmx).mechanic,
                lane_rules(LaneKind::Horse).mechanic,
                lane_rules(LaneKind::Ninja).mechanic,
                lane_rules(LaneKind::Moon).mechanic,
                lane_rules(LaneKind::Saturn).mechanic,
                lane_rules(LaneKind::Submarine).mechanic,
                lane_rules(LaneKind::Desert).mechanic,
                lane_rules(LaneKind::Time).mechanic,
            ],
        );
        assert_distinct(
            "micro catch profile",
            &[
                catch_rules(CatchKind::Glyph).mechanic,
                catch_rules(CatchKind::Poker).mechanic,
                catch_rules(CatchKind::Pinball).mechanic,
                catch_rules(CatchKind::Tennis).mechanic,
                catch_rules(CatchKind::Cricket).mechanic,
                catch_rules(CatchKind::Alien).mechanic,
                catch_rules(CatchKind::Astro).mechanic,
                catch_rules(CatchKind::Castle).mechanic,
                catch_rules(CatchKind::Potion).mechanic,
            ],
        );
        assert_distinct(
            "micro quest profile",
            &[
                quest_rules(QuestKind::Checkmate).mechanic,
                quest_rules(QuestKind::Cipher).mechanic,
                quest_rules(QuestKind::Marble).mechanic,
                quest_rules(QuestKind::Quantum).mechanic,
                quest_rules(QuestKind::Go).mechanic,
                quest_rules(QuestKind::Pirate).mechanic,
                quest_rules(QuestKind::Samurai).mechanic,
                quest_rules(QuestKind::Mars).mechanic,
                quest_rules(QuestKind::DeepSea).mechanic,
                quest_rules(QuestKind::Volcano).mechanic,
                quest_rules(QuestKind::Jungle).mechanic,
                quest_rules(QuestKind::Dragon).mechanic,
                quest_rules(QuestKind::Mirror).mechanic,
            ],
        );
    }

    fn assert_distinct(label: &str, values: &[&str]) {
        let mut seen = HashSet::new();
        for value in values {
            assert!(seen.insert(*value), "duplicate {label}: {value}");
        }
    }

    fn game_mechanic_label(game: &GameInfo) -> String {
        match game.kind {
            GameKind::Snake => "snake-growth".to_string(),
            GameKind::Tetris => "falling-polyomino-well".to_string(),
            GameKind::Pong => "paddle-duel".to_string(),
            GameKind::TronCycles => "hard-light-cycle-duel".to_string(),
            GameKind::TronGridRun => "solo-fading-trail-core-route".to_string(),
            GameKind::Invaders => "shielded-invader-shooter".to_string(),
            GameKind::Missile => "reticle-city-defense".to_string(),
            GameKind::Breakout => "brick-paddle-breakout".to_string(),
            GameKind::Meteor => falling_rules("Meteor Dodge", 1).mechanic.to_string(),
            GameKind::Racer => "traffic-lane-threading".to_string(),
            GameKind::Frog => "traffic-crossing".to_string(),
            GameKind::Target => "timed-reticle-targets".to_string(),
            GameKind::Coin => "grid-coin-trap-collect".to_string(),
            GameKind::Minefield => grid_rules("Minefield", 1).mechanic.to_string(),
            GameKind::Maze => grid_rules("Maze Runner", 1).mechanic.to_string(),
            GameKind::Whack => "cursor-pop-targets".to_string(),
            GameKind::Simon => "memory-sequence-repeat".to_string(),
            GameKind::Reaction => "single-key-reaction".to_string(),
            GameKind::Flappy => "up-down-gate-threading".to_string(),
            GameKind::Asteroid => scroll_rules("Asteroid Belt").mechanic.to_string(),
            GameKind::Star => falling_rules("Star Catcher", 1).mechanic.to_string(),
            GameKind::Laser => "falling-block-shooter".to_string(),
            GameKind::Dungeon => "enemy-treasure-stairs".to_string(),
            GameKind::River => scroll_rules("River Raid").mechanic.to_string(),
            GameKind::Memory => "tile-pair-memory".to_string(),
            GameKind::Number => "timed-arithmetic".to_string(),
            GameKind::Circuit => grid_rules("Circuit Trace", 1).mechanic.to_string(),
            GameKind::Orbit => "rotating-core-guard".to_string(),
            GameKind::BlockDrop => falling_rules("Block Drop", 1).mechanic.to_string(),
            GameKind::CometCatcher => falling_rules("Comet Catcher", 1).mechanic.to_string(),
            GameKind::BombSweeper => grid_rules("Bomb Sweeper", 1).mechanic.to_string(),
            GameKind::NeonDrift => scroll_rules("Neon Drift").mechanic.to_string(),
            GameKind::CargoCatch => falling_rules("Cargo Catch", 1).mechanic.to_string(),
            GameKind::GemRush => falling_rules("Gem Rush", 1).mechanic.to_string(),
            GameKind::TrapRunner => grid_rules("Trap Runner", 1).mechanic.to_string(),
            GameKind::ReactorTrace => grid_rules("Reactor Trace", 1).mechanic.to_string(),
            GameKind::DroneDodge => scroll_rules("Drone Dodge").mechanic.to_string(),
            GameKind::PearlDiver => falling_rules("Pearl Diver", 1).mechanic.to_string(),
            GameKind::SolarSailer => scroll_rules("Solar Sailer").mechanic.to_string(),
            GameKind::VaultEscape => grid_rules("Vault Escape", 1).mechanic.to_string(),
            GameKind::DataStorm => falling_rules("Data Storm", 1).mechanic.to_string(),
            GameKind::PixelPop => "cluster-pop-puzzle".to_string(),
            GameKind::BugHunt => "crawler-shooter".to_string(),
            GameKind::FuelRun => scroll_rules("Fuel Run").mechanic.to_string(),
            GameKind::SparkChase => scroll_rules("Spark Chase").mechanic.to_string(),
            GameKind::IceSlide => grid_rules("Ice Slide", 1).mechanic.to_string(),
            GameKind::SignalTrace => grid_rules("Signal Trace", 1).mechanic.to_string(),
            GameKind::OrbitalCourier => scroll_rules("Orbital Courier").mechanic.to_string(),
            GameKind::RainRunner => falling_rules("Rain Runner", 1).mechanic.to_string(),
            GameKind::ByteBlaster => "falling-letter-typing".to_string(),
            GameKind::StormSurge => scroll_rules("Storm Surge").mechanic.to_string(),
            GameKind::CrystalCavern => grid_rules("Crystal Cavern", 1).mechanic.to_string(),
            GameKind::TicTacToe => "tic-tac-toe-cpu".to_string(),
            GameKind::Chess => "legal-chess-cpu".to_string(),
            GameKind::Checkers => "mandatory-capture-checkers".to_string(),
            GameKind::Micro(index) => {
                format!("micro-{}", micro_mode_label(MICRO_GAMES[index].mode))
            }
        }
    }

    fn micro_mode_label(mode: MicroMode) -> String {
        match mode {
            MicroMode::ConnectFour => "connect-four".to_string(),
            MicroMode::WordGuess(WordKind::Vault) => "word-vault".to_string(),
            MicroMode::WordGuess(WordKind::Hangman) => "hangman".to_string(),
            MicroMode::Blackjack => "blackjack-table".to_string(),
            MicroMode::BlackjackBlitz => "blackjack-blitz".to_string(),
            MicroMode::Battleship => "battleship".to_string(),
            MicroMode::TowerStack => "tower-stack".to_string(),
            MicroMode::LightsOut => "lights-out".to_string(),
            MicroMode::SlidePuzzle => "slide-puzzle".to_string(),
            MicroMode::DominoChain => "domino-chain".to_string(),
            MicroMode::MiniGolf => "mini-golf".to_string(),
            MicroMode::Darts => "darts".to_string(),
            MicroMode::Mancala => "mancala".to_string(),
            MicroMode::MiniSudoku => "mini-sudoku".to_string(),
            MicroMode::Reversi => "reversi".to_string(),
            MicroMode::Bowling => "bowling".to_string(),
            MicroMode::SkeeBall => "skee-ball".to_string(),
            MicroMode::Keeper => "keeper".to_string(),
            MicroMode::Quest(kind) => quest_rules(kind).mechanic.to_string(),
            MicroMode::Lane(kind) => lane_rules(kind).mechanic.to_string(),
            MicroMode::Catch(kind) => catch_rules(kind).mechanic.to_string(),
            MicroMode::Aim(kind) => aim_rules(kind).mechanic.to_string(),
            MicroMode::Sequence(kind) => sequence_rules(kind, 1).mechanic.to_string(),
        }
    }
}
