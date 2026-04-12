void Main() {
    int[4] a = [1, 2, 3, 4];
    int[4] b = [-1001, 1002, -1003, 1004];
    int[4] c = [0, 500, -1000, 1005];

    int[4] d = a + b;
    int[4] e = ((d >= c) & d) * a;

    for (int i = 0; i < 4; i++) {
        iout(4, e[i]);
    }

    return;
}