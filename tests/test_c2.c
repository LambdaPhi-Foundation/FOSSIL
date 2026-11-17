int main() { int a = 10; int b; asm("mov %0, %%rax" : "=r"(b) : "r"(a)); return b; }
