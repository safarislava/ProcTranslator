char gen(int i) {
    if (i % 5 == 0) {
        return 'a';
    }
    else if (i % 5 == 1) {
        return 'b';
    }
    else if (i % 5 == 2) {
        return 'c';
    }
    else if (i % 5 == 3) {
        return 'd';
    }
    else if (i % 5 == 4) {
        return 'e';
    }
    return '\0';
}

char Main() {
    char[10] a = new char[10];
    for (int i = 0; i < 9; i++) {
        a[i] = gen(i);
    }
    return a[9];
}