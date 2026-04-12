void Main() {
    int[16] a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    int[16] b = [5, 2, 3, 4, 5, 9, 7, 8, 9, 10, 13, 1, 13, 14, 15, 3];
    int[16] c = new int[16];

    for (int i = 0; i < 4; i++) {
        for (int j = 0; j < 4; j++) {
            int sum = 0;
            for (int k = 0; k < 4; k++) {
                sum += a[i * 4 + k] * b[k * 4 + j];
            }
            c[i * 4 + j] = sum;
        }
    }

    for (int i = 0; i < 16; i++) {
        iout(4, c[i]);
    }
    return;
}