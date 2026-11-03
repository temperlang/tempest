package myers_diff;

import java.util.List;

public final class Patch<T> {
    public final List<Change<T>> changes;

    public Patch(List<Change<T>> changes) {
        this.changes = changes;
    }

    public List<Change<T>> getChanges() {
        return this.changes;
    }
}
