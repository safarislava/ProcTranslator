void Main() {
    int[4] a = new int[4];
    a[0] = 52;
    a[1] = 5;
    a[2] = 1023;
    a[3] = 746;

    int[4] b = new int[4];
    b[0] = 7;
    b[1] = 4;
    b[2] = -2;
    b[3] = -1;

    int[4] c = a & b;
    for (int i = 0; i < 4; i++) {
        iout(4, c[i]);
    }

    return;
}