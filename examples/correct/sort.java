Data data = new Data();

class Data {
    int size = 0;
    int[100] buffer = new int[100];
    int pointer = 0;

    void add(int value) {
        this.buffer[this.pointer++] = value;
        return;
    }

    void sort() {
        if (this.size <= 1) {
            return;
        }
        this.quickSort(0, this.size - 1);
        return;
    }

    void quickSort(int low, int high) {
        if (low >= high) {
            return;
        }

        int mid = low + (high - low) / 2;
        int pivot = this.buffer[mid];

        int i = low;
        int j = high;

        while (i <= j) {
            while (this.buffer[i] < pivot) {
                i++;
            }
            while (this.buffer[j] > pivot) {
                j--;
            }
            if (i <= j) {
                int temp = this.buffer[i];
                this.buffer[i] = this.buffer[j];
                this.buffer[j] = temp;
                i++;
                j--;
            }
        }

        this.quickSort(low, j);
        this.quickSort(i, high);
        return;
    }

    void print() {
        for (int i = 0; i < this.size; i++) {
            iout(4, this.buffer[i]);
        }
        return;
    }
}

void Main() {
    while (data.size == 0 || data.size != data.pointer) {}
    data.sort();
    data.print();
    return;
}

void interrupt0() {
    if (data.size == 0) {
        data.size = iin(0);
    }
    else {
        data.add(iin(0));
    }
    return;
}