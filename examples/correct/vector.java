void Main() {
    int[4] a = new int[4];
    int[4] b = new int[4];
    int[4] c = new int[4];

    a[0] = 1;
    a[1] = 2;
    a[2] = 3;
    a[3] = 4;

    b[0] = -1001;
    b[1] = 1002;
    b[2] = -1003;
    b[3] = 1004;

    c[0] = 0;
    c[1] = 500;
    c[2] = -1000;
    c[3] = 1005;

    int[4] d = a + b;
    int[4] e = ((d >= c) & d) * a;

    for (int i = 0; i < 4; i++) {
        iout(4, e[i]);
    }

    return;
}