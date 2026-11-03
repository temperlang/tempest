package renamed;

public final class Checked {
    public final int end;

    public Checked(int end) {
        this.end = end;
    }

    public int delete() {
        return this.end * 2;
    }

    public int getLocal() {
        return this.end + RenamedGlobal.native_;
    }

    public int getEnd() {
        return this.end;
    }
}
