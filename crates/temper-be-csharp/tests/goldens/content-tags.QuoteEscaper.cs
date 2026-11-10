namespace ContentTags;

public class QuoteEscaper : IEscaper
{
    public QuoteEscaper()
    {
    }

    public static QuoteEscaper Instance
    {
        get
        {
            return ContentTagsGlobal.quoteEscaperInstance;
        }
    }

    public string ApplyString(string x)
    {
        return "(" + x + ")";
    }
}
