fn main() {
    test_addition();
    test_loop();
    test_conditional();
    test_method();
    test_error_handling();
    test_enum();
    println!("Hello, world!");
}

// Test addition
fn test_addition() {
    let a = 5;
    let b = 7;
    let result = a + b;
    println!("Addition result: {}", result);
}

// Test loop
fn test_loop() {
    let mut sum = 0;

    for i in 1..=5 {
        sum += i;
    }
    println!("Sum after for loop: {}", sum);

    sum = 0;
    while sum <= 5 {
        sum += 1;
    }
    println!("Sum after while loop: {}", sum);
}

// Test conditional
fn test_conditional() {
    let x = 10;
    let y = 15;

    let result = if x > y {
        x
    } else if y > x {
        y
    } else {
        x + y
    };
    println!("Conditional result: {}", result);
}

// Test method (using a helper function)
fn test_method() {
    let result = multiply(3, 4);
    println!("Multiplication result: {}", result);
}

fn multiply(a: i32, b: i32) -> i32 {
    a * b
}

// Test error handling with Result and Option
fn test_error_handling() {
    match divide(10, 0) {
        Ok(result) => println!("Division result: {}", result),
        Err(e) => println!("An error occurred: {}", e),
    }
}

fn divide(a: i32, b: i32) -> Result<i32, String> {
    if b == 0 {
        Err(String::from("Cannot divide by zero"))
    } else {
        Ok(a / b)
    }
}

// Test enum usage with a match statement
fn test_enum() {
    let day = Day::Monday;

    match day {
        Day::Monday => println!("It's Monday!"),
        Day::Tuesday => println!("It's Tuesday!"),
        _ => println!("It's some other day."),
    }

    // Using if let to check for a specific day
    if let Day::Wednesday = day {
        println!("It's Wednesday!");
    } else {
        println!("It's not Wednesday.");
    }
}

// Define an enum for days of the week
enum Day {
    Monday,
    Tuesday,
    Wednesday,
    Thursday,
    Friday,
    Saturday,
    Sunday,
}

// Define a struct example
struct Point {
    x: i32,
    y: i32,
}

// Implement methods for the struct
impl Point {
    fn new(x: i32, y: i32) -> Self {
        Self { x, y }
    }

    fn distance_from_origin(&self) -> f64 {
        ((self.x.pow(2) + self.y.pow(2)) as f64).sqrt()
    }
}

fn my_function() {
    let condition = true;
    if condition {
        println!("Here");
    } else if condition {
        println!("here");
    } else {
        println!("oops");
    }
}
