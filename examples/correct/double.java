void Main() {
    int a1 = -1;
    int a2 = 0;

    int b1 = 1;
    int b2 = 0;

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

    iout(4, c1);
    iout(4, c2);
    return;
}