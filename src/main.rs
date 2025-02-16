mod app;
mod ram;
mod err;
mod ops;
mod cpu;
mod alu;

fn main()  {
    cpu::run();
}