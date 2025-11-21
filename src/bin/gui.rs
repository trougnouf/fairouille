use rustache::model::Task;
// We can import shared logic, but NOT rustache::ui

fn main() {
    println!("Starting Rustache GUI...");
    // GUI init code will go here
    let t = Task::new("Test shared logic");
    println!("Created task from shared lib: {}", t.summary);
}
