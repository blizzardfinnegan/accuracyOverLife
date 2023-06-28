use derivative::Derivative;
use once_cell::sync::Lazy;

#[derive(Eq,Derivative,Debug)]
#[derivative(PartialEq, Hash)]
pub enum Command{
    Quit,
    StartBP,
    CheckBPState,
    LifecycleMenu,
    BrightnessMenu,
    BrightnessLow,
    BrightnessHigh,
    ReadTemp,
    UpMenuLevel,
    RedrawMenu,
    Login,
    DebugMenu,
    Newline,
    Reboot,
    GetSerial,
    Boot,
}
pub const COMMAND_MAP:Lazy<HashMap<Command,&str>> = Lazy::new(||HashMap::from([
    (Command::Quit, "q\n"),
    (Command::StartBP, "N"),
    (Command::CheckBPState, "n"),
    (Command::LifecycleMenu, "L"),
    (Command::BrightnessMenu, "B"),
    (Command::BrightnessHigh, "0"),
    (Command::BrightnessLow, "1"),
    (Command::ReadTemp, "H"),
    (Command::UpMenuLevel, "\\"),
    (Command::Login,"root\n"),
    (Command::RedrawMenu,"?"),
    (Command::DebugMenu,"python3 -m debugmenu\n"),
    (Command::Newline,"\n"),
    (Command::Reboot,"shutdown -r now\n"),
    (Command::Boot,"boot\n"),
    (Command::GetSerial,"echo 'y1q' | python3 -m debugmenu\n"),
]));
