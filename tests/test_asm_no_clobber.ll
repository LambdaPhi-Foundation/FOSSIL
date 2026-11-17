declare i32 @puts(i8*)
declare i32 @putchar(i32)

define i32 @main() {
entry:
    %0 = alloca i32
    store i32 10, ptr %0
    %1 = alloca i32
    store i32 20, ptr %1
    %2 = alloca i32
    store i32 0, ptr %2
    %3 = load i32, ptr %2
    %4 = load i32, ptr %0
    %5 = load i32, ptr %2
    %6 = load i32, ptr %1
    call void asm sideeffect "mov $0, $1\\0Aadd $2, $3", "r,r,r,r,~{dirflag},~{fpsr},~{flags}#intel"(i32 %3, i32 %4, i32 %5, i32 %6)
    %7 = load i32, ptr %2
    ret i32 %7
}

