// VietFlow - Ultra-High Performance Vietnamese Input Engine for .NET 10
// Licensed under the GNU General Public License v3.0 (GPLv3)

using System.Diagnostics;
using System.Text;
using VietFlow;
using VietFlow.Charsets;
using VietFlow.Engine;

Console.OutputEncoding = Encoding.UTF8;
Console.WriteLine("=================================================================");
Console.WriteLine(" VietFlow - Ultra-High Performance Vietnamese Input Engine (.NET 10)");
Console.WriteLine("=================================================================\n");

// 1. Showcase different input methods
ShowcaseMethod("Telex", InputMethod.Telex, "Tieengs Vieetj treen neefn tangr .NET 10");
ShowcaseMethod("Simple Telex", InputMethod.SimpleTelex, "ddas banhs web wiki");
ShowcaseMethod("VNI", InputMethod.Vni, "Tie6ng1 Vie6t5 tre6n ne6n2 tang3 .NET 10");
ShowcaseMethod("VIQR", InputMethod.Viqr, "Tie^ng' Vie^.t tre^n ne^`n ta?ng .NET 10");

// 2. Modern vs Classic Tone Styles
Console.WriteLine("--- Modern vs Classic Tone Placement ---");
var classicSession = new VietFlowSession(InputMethod.Telex, modernStyle: false);
var modernSession = new VietFlowSession(InputMethod.Telex, modernStyle: true);

Console.WriteLine($"Classic (hoaf, tuyf):  {classicSession.ProcessString("hoaf tuyf")}");
Console.WriteLine($"Modern  (hoaf, tuyf):  {modernSession.ProcessString("hoaf tuyf")}\n");

// 3. Macro Expansion
Console.WriteLine("--- Macro Expansion ---");
var macroSession = new VietFlowSession(InputMethod.Telex);
macroSession.Macros.AddOrUpdate("vf", "VietFlow");
macroSession.Macros.AddOrUpdate("vn", "Việt Nam");
Console.WriteLine($"Typed: 'vf laf engine gox tieengs vn nhanh nhaats.'");
Console.WriteLine($"Result: {macroSession.ProcessString("vf laf engine gox tieengs vn nhanh nhaats.")}\n");

// 4. Performance Benchmark
Console.WriteLine("--- Throughput & Performance Benchmark (.NET 10) ---");
const int iterations = 100_000;
const string sampleInput = "Tieengs Vieetj voosn laf tieengs meej ddeer cuar daan toocj Vieetj Nam.";

var session = new VietFlowSession(InputMethod.Telex);

// Warmup
for (int i = 0; i < 5_000; i++)
{
    _ = session.ProcessString(sampleInput);
}

long totalKeystrokes = (long)sampleInput.Length * iterations;
long allocBefore = GC.GetAllocatedBytesForCurrentThread();
var sw = Stopwatch.StartNew();

for (int i = 0; i < iterations; i++)
{
    _ = session.ProcessString(sampleInput);
}

sw.Stop();
long allocAfter = GC.GetAllocatedBytesForCurrentThread();

double elapsedSec = sw.Elapsed.TotalSeconds;
double keystrokesPerSec = totalKeystrokes / elapsedSec;
long totalBytesAllocated = allocAfter - allocBefore;
double bytesPerIteration = (double)totalBytesAllocated / iterations;

Console.WriteLine($"Sample Input:       \"{sampleInput}\"");
Console.WriteLine($"Keystrokes Typed:   {totalKeystrokes:N0}");
Console.WriteLine($"Elapsed Time:       {sw.ElapsedMilliseconds} ms ({elapsedSec:F3} s)");
Console.WriteLine($"Throughput:         {keystrokesPerSec:N0} keystrokes/sec ({keystrokesPerSec / 1_000_000.0:F2} M keystrokes/s)");
Console.WriteLine($"Memory Allocated:   {totalBytesAllocated:N0} bytes ({bytesPerIteration:F1} bytes/string iteration)");

// UTF-8 Pipeline Benchmark
Console.WriteLine("\n--- Zero-Allocation UTF-8 Byte Stream Benchmark ---");
byte[] utf8Input = Encoding.UTF8.GetBytes(sampleInput);
byte[] utf8Output = new byte[utf8Input.Length * 2];

long utf8AllocBefore = GC.GetAllocatedBytesForCurrentThread();
sw.Restart();

for (int i = 0; i < iterations; i++)
{
    _ = session.ProcessUtf8(utf8Input, utf8Output);
}

sw.Stop();
long utf8AllocAfter = GC.GetAllocatedBytesForCurrentThread();
long utf8Alloc = utf8AllocAfter - utf8AllocBefore;

Console.WriteLine($"UTF-8 Pipeline:     {totalKeystrokes / sw.Elapsed.TotalSeconds:N0} keystrokes/sec");
Console.WriteLine($"UTF-8 Heap Alloc:   {utf8Alloc} bytes total ({(double)utf8Alloc / iterations:F2} bytes/call)");
Console.WriteLine("=================================================================");

static void ShowcaseMethod(string name, InputMethod method, string input)
{
    var sess = new VietFlowSession(method);
    string result = sess.ProcessString(input);
    Console.WriteLine($"[{name,-12}] Input:  \"{input}\"");
    Console.WriteLine($"               Output: \"{result}\"\n");
}
