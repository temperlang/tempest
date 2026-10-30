package setters;

public final class ConcreteWhatever implements Whatever {
    public int something;

    public ConcreteWhatever(int something) {
        this.something = something;
    }

    public int getSomething() {
        return this.something;
    }

    public void setSomething(int newSomething) {
        this.something = newSomething;
    }
}
