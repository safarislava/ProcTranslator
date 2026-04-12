int calc(int a, int b, int c) {
    return (a + b) * c;
}

void Main() {
    int a = 9;
    int b = -1;
    int res = calc(a, b, 4);
    iout(4, res);
    return;
}