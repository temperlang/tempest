package defaults;

import temper.core.Nullable;

final class Counter {
    public int count;

    public Counter(@Nullable Integer start) {
        int start2 = start != null ? start : 10;
        this.count = start2;
    }

    public Counter() {
        this(null);
    }

    public int add(@Nullable Integer by) {
        int by2 = by != null ? by : 1;
        this.count = this.count + by2;
        return this.count;
    }

    public int add() {
        return add(null);
    }

    public int getCount() {
        return this.count;
    }

    public void setCount(int newCount) {
        this.count = newCount;
    }
}
