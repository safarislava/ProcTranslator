bool flag = true;

void interrupt1() {
    char c = in(2);
    if (c == '\0') {
        flag = false;
    }
    out(5, c);
    return;
}

void Main() {
    while (flag) {}
    return;
}