void Main() {
    int[16] a = [1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16];
    int[16] b = [5, 2, 3, 4, 5, 9, 7, 8, 9, 10, 13, 1, 13, 14, 15, 3];
    int[16] c = new int[16];

    for (int i = 0; i < 4; i++) {
        int[4] row = a[i*4:4];
        int[4] sum = [0, 0, 0, 0];

        for (int k = 0; k < 4; k++) {
            int s = row[k];
            int[4] t = new int[4];
            for (int j = 0; j < 4; j++) {
                t[j] = s;
            }
            sum += t * b[k*4:4];
        }
        c[i*4:4] = sum;
    }

    for (int i = 0; i < 16; i++) {
        iout(4, c[i]);
    }
    return;
}