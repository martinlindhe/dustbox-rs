// some code to objdump, to study calling conventions

#include <stdio.h>

void a_void_func() {
    int a = 10;
    printf("%d", a);
}

void b_void_func() {
    int a = 10;
}

int main(int argc, char **argv) {
    a_void_func();
    b_void_func();
}
