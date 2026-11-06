package libraries;

public final class Carry {
    public boolean carrying;
    public int state;

    public Carry(int start) {
        this.carrying = false;
        this.state = 0;
        if (start != 0) {
            this.carrying = true;
            this.state = start;
        }
    }

    public boolean isCarrying() {
        return this.carrying;
    }

    public int getState() {
        return this.state;
    }
}
