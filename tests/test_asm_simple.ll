declare i32 @puts(i8*)
declare i32 @putchar(i32)

define i32 @main() {
entry:
    %0 = alloca i32
    store i32 10, ptr %0
    %1 = load i32, ptr %0
    call void asm sideeffect "mov rax, $0", "r,~{rax},~{dirflag},~{fpsr},~{flags}#intel"(i32 %1)
    ret i32 0
}

