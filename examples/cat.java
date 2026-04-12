bool flag = true;

void interrupt1() {
    char c = cin(2);
    if (c == '\0') {
        flag = false;
    }
    cout(5, c);
    return;
}

void Main() {
    while (flag) {}
    return;
}