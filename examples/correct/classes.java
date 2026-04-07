class Node {
    int counter = 5;

    int Increment() {
        this.counter = this.counter + 2;
        return this.counter;
    }
}

void Main() {
    Node node = new Node();
    node.Increment();
    node.Increment();
    iout(4, node.counter);
    return;
}
