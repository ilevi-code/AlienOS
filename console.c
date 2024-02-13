#include "console.h"

static volatile unsigned int * const uart0 = (unsigned int *)0x09000000;

void write_uart0(const char *s) {
    while(*s != '\0') {
        *uart0 = (unsigned int)(*s);
        s++;
    }
}

void panic(const char* s) {
    write_uart0("panicked: ");
    write_uart0(s);
    for(;;);
}

void print_int(int num) {
    char dec_str[32] = {0};
    int i = 0;
    int is_negative = 0;

    // Convert each digit of the integer to a character
    do {
        dec_str[i++] = num % 10 + '0';
        num /= 10;
    } while (num != 0);

    // Null-terminate the string
    dec_str[i] = '\0';

    // Reverse the string
    int len = i;
    for (i = 0; i < len / 2; i++) {
        char temp = dec_str[i];
        dec_str[i] = dec_str[len - i - 1];
        dec_str[len - i - 1] = temp;
    }
    write_uart0(dec_str);
}

void print_hex(int num) {
    char hex_str[32] = {0};
    int i = 0;
    while (num != 0) {
        int rem = num % 16;
        hex_str[i++] = (rem < 10) ? (rem + '0') : (rem + 'A' - 10);
        num = num / 16;
    }
    hex_str[i] = '\0';

    // Reverse the string
    int len = i;
    for (i = 0; i < len / 2; i++) {
        char temp = hex_str[i];
        hex_str[i] = hex_str[len - i - 1];
        hex_str[len - i - 1] = temp;
    }
    write_uart0(hex_str);
}
