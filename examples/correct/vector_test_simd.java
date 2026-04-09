void Main() {
    int seed = 42;
    int[100] a = new int[100];
    for (int i = 0; i < 100; i++) {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        a[i] = seed;
    }

    int[100] b = new int[100];
    for (int i = 0; i < 100; i++) {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        b[i] = seed;
    }

    int[100] c = new int[100];
    for (int i = 0; i < 100; i += 4) {
        c[i:4] = a[i:4] + b[i:4];
    }

    for (int i = 0; i < 100; i++) {
        iout(4, c[i]);
    }
    return;
}