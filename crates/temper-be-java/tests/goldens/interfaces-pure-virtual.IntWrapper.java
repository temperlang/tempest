package pure_virtual;

final class IntWrapper implements Wrapper<Integer> {
    public IntWrapper() {}

    private int wrapUnboxed(int item) {
        return item + 1;
    }

    public Integer wrap(Integer item) {
        return wrapUnboxed(item.intValue());
    }
}
