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
        for (int k = 0; k < 8; k++) {
            int s = a[i * 8 + k];

            int[4] mask = new int[4];
            for (int j = 0; j < 4; j++) {
                mask[j] = s;
            }

            for (int j = 0; j < 8; j += 4) {
                c[i * 8 + j : 4] += mask * b[k * 8 + j : 4];
            }
        }
    }

    for (int i = 0; i < 64; i++) {
        iout(4, c[i]);
    }
    return;
}