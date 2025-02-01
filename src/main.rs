mod app;

use pixels::Error;

fn main() -> Result<(), Error> {
    app::run_loop()
}