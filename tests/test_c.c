int main() { int a = 10; asm("mov %%rax, %0" : "=r"(a)); return a; }
