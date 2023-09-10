extern crate colored;
use colored::*;

// painstakeingly hand-crafted ASCII art coloring
// because street culture is life
#[rustfmt::skip]
pub fn print_banner() {
    println!("{}{}{}",    "       __".red().bold(), "       __".green().bold(),"                  __  .__    ".blue().bold());
    println!("{}{}{}{}{}",  "      |__|".red().bold(),"__ __".yellow().bold(), "|  | __".green().bold()," ____ ".cyan().bold(),  "  _____/  |_|  |   ".blue().bold());
    println!("{}{}{}{}{}",  "      |  |".red().bold(),"  |  \\".yellow().bold(),"  |/ /".green().bold(),"/ __ \\".cyan().bold(), "_/ ___\\   __\\  |   ".blue().bold());
    println!("{}{}{}{}{}",  "      |  |".red().bold(),"  |  /".yellow().bold(),"    <".green().bold(), "\\  ___/".cyan().bold(), "\\  \\___|  | |  |__ ".blue().bold());
    println!("{}{}{}{}{}", "  /\\__|  |".red().bold(),"____/".yellow().bold(), "|__|_ \\".green().bold(),"\\___  >".cyan().bold(),"\\___  >__| |____/ ".blue().bold());
    println!("{}{}{}{}{}", "  \\______|".red().bold(),"     ".yellow().bold(), "     \\/ ".green().bold(),"   \\/ ".cyan().bold(),"    \\/            ".blue().bold());
}
