// The box, a Sith tomb (SJK; see README.md), and its lid: black basalt lit by the world's
// light as a player model is, through its relief; what is cut into it glows red on top,
// pulsing slowly, and in the dynamic glow.
models/sjk/mystery_box
{
	q3map_nolightmap
	{
		map models/sjk/mystery_box
		normalMap models/sjk/mystery_box_n
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/mystery_box_glow
		blendFunc add
		rgbGen wave sin 0.8 0.2 0 0.4
		glow
	}
}

// The faint red halo of light rising over the tomb, so a box is found from afar (SJK; see
// README.md): added, seen from inside and out, its rays drifting slowly round, breathing.
models/sjk/mystery_box_halo
{
	cull none
	nopicmip
	{
		map models/sjk/mystery_box_halo
		blendFunc add
		rgbGen wave sin 0.85 0.15 0 0.35
		tcMod scroll 0.02 0
	}
}

// The fire an open tomb lets out (SJK; see README.md): two sleeves of red tongues, added,
// seen from inside and out, sweeping round against each other's grain and flickering.
models/sjk/mystery_box_fire
{
	cull none
	nopicmip
	{
		map models/sjk/mystery_box_fire
		blendFunc add
		rgbGen wave noise 0.75 0.25 0 9
		tcMod scroll 0.11 0
		glow
	}
	{
		map models/sjk/mystery_box_fire
		blendFunc add
		rgbGen wave sin 0.5 0.2 0 1.3
		tcMod scale -1.5 1
		tcMod scroll 0.07 0
	}
}

// The embers inside an open tomb (SJK; see README.md): two layers of red veins, added,
// crawling across each other.
models/sjk/mystery_box_ember
{
	cull none
	{
		map models/sjk/mystery_box_ember
		blendFunc add
		rgbGen wave sin 0.8 0.2 0 0.6
		tcMod scroll 0.03 0.02
		glow
	}
	{
		map models/sjk/mystery_box_ember
		blendFunc add
		tcMod scale 1.7 1.7
		tcMod scroll -0.02 0.035
	}
}
// The teddy bear a box stops on when it blows up (SJK; see README.md): plush, lit as a
// player model is.
models/sjk/teddy_bear
{
	{
		map models/sjk/teddy_bear
		rgbGen lightingDiffuse
	}
}

// The box's loot of 11/11/2026 (SJK; see README.md, scripts/box_loot_model.py): the
// piles of gold coins (the Ray Gun too until 20/11/2026), lit as a player model is, with a glint that slides
// over the metal as the eye moves.
models/sjk/box_loot
{
	{
		map models/sjk/box_loot
		rgbGen lightingDiffuse
	}
	{
		map models/sjk/box_loot
		blendFunc GL_DST_COLOR GL_ONE
		tcGen environment
		rgbGen const ( 0.35 0.35 0.35 )
	}
}
