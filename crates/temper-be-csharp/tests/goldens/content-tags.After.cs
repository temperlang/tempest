using System.Collections.Generic;

namespace ContentTags;

public class After
{
    private readonly IReadOnlyList<IEffect> effects;
    private readonly bool stateAfter;

    public After(IReadOnlyList<IEffect> effects, bool stateAfter)
    {
        this.effects = effects;
        this.stateAfter = stateAfter;
    }

    public IReadOnlyList<IEffect> Effects
    {
        get
        {
            return this.effects;
        }
    }

    public bool StateAfter
    {
        get
        {
            return this.stateAfter;
        }
    }
}
