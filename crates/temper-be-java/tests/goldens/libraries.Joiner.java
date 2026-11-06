package libraries;

import java.util.List;
import temper.core.Nullable;

public final class Joiner {
    public String out;

    public Joiner() {
        this.out = "";
    }

    public void addWord(Word w) {
        this.out = this.out + "w" + w.text;
    }

    public void addFlags(List<Boolean> x) {
        this.out = this.out + "f" + x.size();
    }

    public void addTexts(List<String> x) {
        this.out = this.out + "t" + x.size();
    }

    public void addMaybes(List<@Nullable String> x) {
        this.out = this.out + "m" + x.size();
    }

    public String getOut() {
        return this.out;
    }

    public void setOut(String newOut) {
        this.out = newOut;
    }
}
