using System;

namespace ContentTags;

public class Picker
{
    private readonly bool strict;

    public Picker(bool strict)
    {
        this.strict = strict;
    }

    public bool Strict
    {
        get
        {
            return this.strict;
        }
    }

    public IEscaper EscaperFor(bool inQuote, Action<string, bool> callout)
    {
        if (this.strict && inQuote)
        {
            callout("a value in quotes", false);
        }
        if (inQuote)
        {
            return new Both(TextEscaper.Instance, QuoteEscaper.Instance);
        }
        else
        {
            return TextEscaper.Instance;
        }
    }
}
