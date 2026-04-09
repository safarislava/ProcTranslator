void sum(int a2, int a1, int b2, int b1) {
    int c1 = a1 + b1;

    int a1hb = a1 >> 63;
    int b1hb = b1 >> 63;
    int c1hb = c1 >> 63;

    int carry = 0;
    if (a1hb == 1 && b1hb == 1) {
        carry = 1;
    }
    else if (a1hb == 1 || b1hb == 1) {
        if (c1hb == 0) {
            carry = 1;
        }
    }

    int c2 = a2 + b2 + carry;

    iout(4, c2);
    iout(4, c1);
    return;
}

void Main() {
    sum(0, -1, 0, 1);
    sum(10, -5, 0, 10);
    sum(5, -9223372036854775808, 5, -9223372036854775808);
    sum(9223372036854775807, -1, 0, 1);
    return;
}