String name = new String();

class String {
    char[100] buffer = new char[100];
    int pointer = 0;

    void add(char v) {
        this.buffer[this.pointer++] = v;
        return;
    }

    bool end() {
        return this.pointer != 0 && this.buffer[this.pointer - 1] == '\0';
    }

    char get(int i) {
        return this.buffer[i];
    }
}

void Main() {
    print_question();

    while (!name.end()) {}

    print_hello();

    return;
}

void print_question() {
    cout(5, 'W');
    cout(5, 'h');
    cout(5, 'a');
    cout(5, 't');
    cout(5, '\'');
    cout(5, 's');
    cout(5, ' ');
    cout(5, 'y');
    cout(5, 'o');
    cout(5, 'u');
    cout(5, 'r');
    cout(5, ' ');
    cout(5, 'n');
    cout(5, 'a');
    cout(5, 'm');
    cout(5, 'e');
    cout(5, '?');
    cout(5, '\n');
    return;
}

void print_name() {
    for (int i = 0; true; i++) {
        char c = name.get(i);
        if (c == '\0') {
            break;
        }
        cout(5, c);
    }
    return;
}

void print_hello() {
    cout(5, 'H');
    cout(5, 'e');
    cout(5, 'l');
    cout(5, 'l');
    cout(5, 'o');
    cout(5, ',');
    cout(5, ' ');
    print_name();
    cout(5, '\n');
    return;
}

void interrupt1() {
    char v = cin(2);
    name.add(v);
    return;
}
