package pure_virtual;

final class BoxWrapper implements Wrapper<Box> {
    public BoxWrapper() {}

    public Box wrap(Box item) {
        item = new Box("[" + item.label + "]");
        return item;
    }
}
