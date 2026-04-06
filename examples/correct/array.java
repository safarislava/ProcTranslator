char Main() {
    char[] a = new char[10];
    char[] b = new char[10];
    for (int i = 0; i < 10; i++) {
        if (i % 2 == 0) {
            a[i] = 'a';
        }
    }
    return a[0];
}