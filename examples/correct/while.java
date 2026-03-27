int f(int x) {
    return x + 5;
}

int Main() {
    int x = 0;

    while (true) {
        if (x == 9) {
            x++;
        }
        else if (x == 10) {
            break;
        }
        x = f(x);
    }

    return x;
}