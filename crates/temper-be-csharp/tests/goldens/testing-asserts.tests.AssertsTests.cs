using Microsoft.VisualStudio.TestTools.UnitTesting;
using System.Collections.Generic;
using TemperLang.Core;
using TemperLang.Std.Testing;

namespace Asserts;

[TestClass]
public class AssertsTests
{
    [TestMethod]
    public void ATestCase()
    {
        Test test = new Test();
        try
        {
            test.Assert(Float64.Compare(AssertsGlobal.NormSquared(new Vec2(0.0, 0.0)), 0.0) == 0, () => "Oh noes");
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void AFailingTest()
    {
        Test test = new Test();
        try
        {
            test.Assert(false, () => "because it's wrong");
            TestHelper helper = new TestHelper(3, 2);
            AssertsGlobal.AssertMoreThings(test, helper, Listed.CreateReadOnlyList<IReadOnlyList<string>>(Listed.CreateReadOnlyList<string>("bye")));
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void Exported()
    {
        Test test = new Test();
        try
        {
            Vec2 vec = new Vec2(1.5, 2.5);
            {
                string actual = vec.ToString();
                string expected = "(1.5, 2.5) size squared 8.5";
                test.Assert(actual == expected, () => "expected vec.toString() == (" + expected + ") not (" + actual + ")");
            }
        }
        finally
        {
            test.SoftFailToHard();
        }
    }

    [TestMethod]
    public void ModuleValues()
    {
        Test test = new Test();
        try
        {
            {
                int length = AssertsGlobal.unitVecs.Count;
                int i = 0;
                while (i < length)
                {
                    Vec2 vec = AssertsGlobal.unitVecs[i];
                    i = i + 1;
                    {
                        double actual = AssertsGlobal.NormSquared(vec);
                        double expected = 1.0;
                        test.Assert(Float64.Compare(actual, expected) == 0, () => "expected normSquared(vec) == (" + AssertsGlobal.FormatFloat64(expected) + ") not (" + AssertsGlobal.FormatFloat64(actual) + ")");
                    }
                }
            }
        }
        finally
        {
            test.SoftFailToHard();
        }
    }
}
