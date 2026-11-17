void foo(int a) { asm("mov %0, %%rax" : : "r"(a) : "rax"); }
