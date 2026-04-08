void Main() {
    int[4] a = new int[4];
    int[4] b = new int[4];

    a[0] = 1;
    a[1] = 2;
    a[2] = 3;
    a[3] = 4;

    b[0] = -1001;
    b[1] = 1002;
    b[2] = -1003;
    b[3] = 1004;

    int[4] c = a + b;

    for (int i = 0; i < 4; i++) {
        iout(4, c[i]);
    }

    return;
}