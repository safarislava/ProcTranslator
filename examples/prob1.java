int len(int n) {
    int c = 0;
    while (n > 0) {
        n = n / 10;
        c++;
    }
    return c;
}

int pow10(int n) {
    int res = 1;
    for (int i = 0; i < n; i++) {
        res *= 10;
    }
    return res;
}

bool check(int n) {
    int l = len(n);
    for (int i = 0; i < l / 2; i++) {
        int a = (n / pow10(i)) % 10;
        int b = (n / pow10(l - i - 1)) % 10;
        if (a != b) {
            return false;
        }
    }
    return true;
}

int calc() {
    bool flag = false;
    for (int i = 999; i >= 900; i--) {
        for (int j = 999; j >= 900; j--) {
            flag = check(i * j);

            if (flag) {
                return i * j;
            }
        }
    }
    return -1;
}

int Main() {
    int res = calc();
    iout(4, res);
    return res;
}