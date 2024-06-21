"""
Directly calls siffio rather than
using the `SiffReader` methods for
a fairer comparison to the `Rust`
code (which doesn't have to deal with
Python either).
"""
import siffpy

from local_consts import small_path, large_path

TEST_RCLONE = False

def test_read_small(sr):
    sr.siffio.get_frames(frames = list(range(40)))

def test_read_large(sr):
    sr.siffio.get_frames(frames = list(range(50000)))

def histogram_from_all_frames(sr):
    sr.siffio.get_histogram()

if __name__ == '__main__':
    import timeit
    sr_large = siffpy.SiffReader(large_path)
    print("Opened large")
    print(
        "Get histogram from all frames:\n",
        timeit.timeit(
            "test(sr)",
            setup="from __main__ import histogram_from_all_frames as test"
            + "\nfrom __main__ import sr_large as sr",
            number = 20,
        )/20 , "sec"
    )
    print(
        "Get 50000 large frames:\n",
        timeit.timeit(
            "test(sr)",
            setup="from __main__ import test_read_large as test"
            + "\nfrom __main__ import sr_large as sr",
            number = 10,
        )/10 , "sec per iter"
    )
    sr_small = siffpy.SiffReader(small_path)
    print("Opened small")
    print(
        "Get 40 small frames:\n",
        timeit.timeit(
            "test(sr)",
            setup="from __main__ import test_read_small as test"
            + "\nfrom __main__ import sr_small as sr",
            number = 100,
        )/100 * 1000 , "msec per iter"
    )