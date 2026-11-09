namespace PropertyMembers;

internal interface ILabeled<L> : IHolder<C>
{
    string Label(L l);
}
