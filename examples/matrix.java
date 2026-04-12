void Main() {
    int seed = 42;
    int[64] a = new int[64];
    for (int i = 0; i < 64; i++) {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        a[i] = seed;
    }

    int[64] b = new int[64];
    for (int i = 0; i < 64; i++) {
        seed = (seed * 1103515245 + 12345) % 2147483648;
        b[i] = seed;
    }

    int[64] c = new int[64];
    for (int i = 0; i < 8; i++) {
        for (int j = 0; j < 8; j++) {
            int sum = 0;
            for (int k = 0; k < 8; k++) {
                sum += a[i * 8 + k] * b[k * 8 + j];
            }
            c[i * 8 + j] = sum;
        }
    }

    for (int i = 0; i < 64; i++) {
        iout(4, c[i]);
    }
    return;
}