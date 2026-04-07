String res = new String();

class String {
    char[100] buffer = new char[100];
    int pointer = 1;

    void init() {
        this.buffer[0] = '\n';
        return;
    }

    void add(char v) {
        buffer[this.pointer++] = v;
        return;
    }

    bool end() {
        return buffer[this.pointer - 1] == '\0';
    }

    char get(int i) {
        return buffer[i];
    }
}

void Main() {
    res.init();

    while (res.end() == false) {}

    print();
    return;
}

void print() {
    for (int i = 1; true; i++) {
        char c = res.get(i);
        cout(5, c);
        if (c == '\0') {
            break;
        }
    }
    return;
}

void interrupt1() {
    char v = cin(2);
    res.add(v);
    return;
}
