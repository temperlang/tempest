package work;

public final class Counter {
    private int count;

    public Counter() {
        this.count = 0;
    }

    public void add(int i) {
        this.count = this.count + i;
    }

    public int total() {
        return this.count + 1000;
    }
}
