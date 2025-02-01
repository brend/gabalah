mod gfx;

use pixels::Error;

fn main() -> Result<(), Error> {
    gfx::run_loop()
}