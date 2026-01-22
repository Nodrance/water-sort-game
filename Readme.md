# Water sort game

It's that one mobile game where you sort water in beakers. Except this is more of a level tester/editor to explore how the game plays/works.

## Features:
- Mouse-only UI. If you want keyboard controls, add them. 
- Copy and paste state. Honestly a text editor is probably faster for editing positions
- Smart text sizing for all UI elements. Makes it as big as possible without overflowing the rectangle, checks height and width. It's supposed to be perfectly centered but I think there's a bit of implicit padding that makes it not quite work
- Add and remove beakers and change their sizes
- Add and remove liquid without pouring. Kinda easy to do by accident if you have a fluid selected and try to select a beaker.
- Supports anything. Any number of beakers of any (or different) size and any number of colors, it even goes past 26 if you edit the code. Colors loop and letters start being double (like "AA"). You can't import or export these though

## Things that would be cool to add:
- Undo
- Randomize colors or make random moves
- Some way to turn off edit mode
- Better rendering code. Right now half of the game uses hardcoded constants. The only things you can safely adjust are the ratio of controls to screen, and the ratio of color buttons to action buttons. And also the list of action buttons.
- Better click detection code. Right now if you change any of the hardcoded spacing constants in one place and not another, the entire game breaks
- Better text code. As mentioned, there's implicit padding I think, just look at the single letters that end up weirdly to the right (some don't??)
- Better rendering code again. I think the padding between each of the beakers is only applied on the left.
- Actual good graphics maybe? Haha no never
- Keyboard controls
- Maybe make the selection stuff easier to understand

## Controls
Click a beaker or color to select it. Click it again to deselect
With a beaker selected, click another beaker to pour or a color to add it
With a color selected, click a beaker to add that color.
Click add or remove to make a new empty beaker to the left of, or delete, the selected beaker. If no beaker is selected, it will do this to the beaker on the end.
Click expand or shrink to increase or decrease the size of the selected beaker
Click copy and paste to copy the current state of the entire board to/from the clipboard. Each row is a beaker, each capital letter is a liquid, and everything else is an empty spot. Empties always end up on top. There is no way to copy or paste individual beakers other than editing in a text editor