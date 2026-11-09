# Star trails

## Lighten

For star trails there is no alignment. A star moves across the frames, and at every pixel STARY keeps the
largest value over all frames. A star that passes over a pixel leaves its brightness there; the dark sky between
stars stays dark. The frames are combined on the **raw sensor values**, one color at a time, and the picture is
demosaiced **once at the end**, which is faster than demosaicing every frame and gives the same trail.

## Comet

A comet has a bright head and a fading tail. STARY gives each frame a weight that falls off over the last
*tail length* frames before keeping the brightest value, so the newest position is the brightest and older
positions fade away. *Reverse order* runs the frames backwards, which turns the tail the other way.

## Gap Filling

When frames are taken with a pause between them, a trail becomes a row of dots. STARY measures how far the stars
moved between consecutive frames (by matching their stars) and adds frames in between, shifting the earlier frame in
small steps along that motion. Only the values clearly above the sky are added, so the background does not brighten.
Things that do not move with the stars, such as the ground, are smeared as well, which is why a
ground mask is recommended.

## The ground

With **Stack ground separately**, the ground is **averaged** from all frames (which reduces its noise)
and blended under the trails through the sky mask.
