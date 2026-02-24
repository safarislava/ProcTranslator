class Node {
    int counter = 5;

    int Increment() {
        return this.counter++;
    }
}

class App {
    void Main() {
        Node node = new Node();
        node.Increment();
        node.Increment();
        return;
    }
}
