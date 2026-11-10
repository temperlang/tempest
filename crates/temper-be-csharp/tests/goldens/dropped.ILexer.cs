namespace Dropped;

internal interface ILexer
{
    string? Scan(string text);

    int Count { get; }
}
