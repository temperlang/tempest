package content_tags;

import java.util.List;

public final class After {
    public final List<Effect> effects;
    public final boolean stateAfter;

    public After(List<Effect> effects, boolean stateAfter) {
        this.effects = effects;
        this.stateAfter = stateAfter;
    }

    public List<Effect> getEffects() {
        return this.effects;
    }

    public boolean isStateAfter() {
        return this.stateAfter;
    }

    public static final class Builder {
        List<Effect> effects;

        public Builder effects(List<Effect> effects) {
            this.effects = effects;
            return this;
        }

        boolean stateAfter;
        boolean stateAfter__set;

        public Builder stateAfter(boolean stateAfter) {
            stateAfter__set = true;
            this.stateAfter = stateAfter;
            return this;
        }

        public After build() {
            if (effects == null || !stateAfter__set) {
                StringBuilder _message = new StringBuilder("Missing required fields:");
                if (effects == null) {
                    _message.append(" effects");
                }
                if (!stateAfter__set) {
                    _message.append(" stateAfter");
                }
                throw new IllegalStateException(_message.toString());
            }
            return new After(effects, stateAfter);
        }
    }
}
