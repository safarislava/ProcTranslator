int res = 0;

void interrupt0() {
    res = in(0);
    return;
}

int Main() {
    while (res == 0) {
    
    }
    return res;
}