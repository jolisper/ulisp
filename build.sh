#!/bin/bash
mkdir -p build/
cargo run --release -- input.ulisp > build/out.asm
nasm -f elf64 build/out.asm
gcc -o build/out build/out.o
