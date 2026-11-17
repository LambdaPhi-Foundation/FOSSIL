declare i32 @puts(i8*)
declare i32 @putchar(i32)

define i32 @main() {
entry:
    %0 = alloca i32
    store i32 10, ptr %0
    %1 = alloca i32
    store i32 0, ptr %1
    %2 = load i32, ptr %1
    %3 = load i32, ptr %0
    call void asm sideeffect "mov $0, $1", "r,r,~{rax},~{dirflag},~{fpsr},~{flags}#intel"(i32 %2, i32 %3)
    ret i32 0
}

