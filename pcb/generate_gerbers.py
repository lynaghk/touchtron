#!/Applications/KiCad/kicad.app/Contents/Frameworks/Python.framework/Versions/Current/bin/python
# See https://docs.kicad-pcb.org/doxygen-python/classpcbnew_1_1MODULE.html
import sys
import os
import re
import csv
sys.path.insert(0, "/Applications/Kicad/kicad.app/Contents/Frameworks/python/site-packages/")
import pcbnew
from pcbnew import *
import shutil


nm_to_mm = 1e-6


def atoi(text):
    return int(text) if text.isdigit() else text

def natural_keys(text):
    return [ atoi(c) for c in re.split(r'(\d+)', text) ]


file_name = sys.argv[1]
output_dir = 'fabrication_output/'
# shutil.rmtree(output_dir, ignore_errors=True)
# os.makedirs(output_dir)


board = pcbnew.LoadBoard(file_name)



##############################
# export centroid file

import json
with open('{}/attrs.json'.format(output_dir), 'r') as f:
    attrs = json.load(f)

with open("{}/centroid.csv".format(output_dir), "w") as f:
    field_names = ["Designator", "Mid X", "Mid Y", "Layer", "Rotation"]
    csv_writer = csv.DictWriter(f, fieldnames=field_names, delimiter=',', quotechar='"', quoting=csv.QUOTE_ALL)
    csv_writer.writeheader()
    for m in sorted(board.GetModules(), key=lambda m: natural_keys(m.GetReference())):
        #Ignore graphics
        if "G***" != m.GetReference():
            c = m.GetCenter()
            d = attrs[m.GetReference()]
            csv_writer.writerow({"Designator": m.GetReference(),
                                 "Mid X": c.x * nm_to_mm,
                                 "Mid Y": -c.y * nm_to_mm,
                                 "Layer": "Top",
                                 "Rotation": (m.GetOrientationDegrees() + int(d.get("LCSC Orientation", "0"))) % 360})

##############################
# export gerbers

gerber_dir = "{}/gerbers/".format(output_dir)
os.makedirs(gerber_dir)

pctl = pcbnew.PLOT_CONTROLLER(board)
popt = pctl.GetPlotOptions()
popt.SetOutputDirectory(gerber_dir)
popt.SetPlotFrameRef(False)
popt.SetLineWidth(pcbnew.FromMM(0.1))

popt.SetAutoScale(False)
popt.SetScale(1)
popt.SetMirror(False)

popt.SetUseGerberAttributes(True)
popt.SetUseGerberProtelExtensions(True)

popt.SetExcludeEdgeLayer(True)
popt.SetUseAuxOrigin(False)
pctl.SetColorMode(True)

popt.SetSubtractMaskFromSilk(False)
popt.SetPlotReference(True)
popt.SetPlotValue(False)

layers = [
    ("F.Cu", pcbnew.F_Cu, "Top layer"),
    ("B.Cu", pcbnew.B_Cu, "Bottom layer"),
    ("F.Paste", pcbnew.F_Paste, "Paste top"),
    ("B.Paste", pcbnew.B_Paste, "Paste bottom"),
    ("F.SilkS", pcbnew.F_SilkS, "Silk top"),
    ("B.SilkS", pcbnew.B_SilkS, "Silk top"),
    ("F.Mask", pcbnew.F_Mask, "Mask top"),
    ("B.Mask", pcbnew.B_Mask, "Mask bottom"),
    ("Edge.Cuts", pcbnew.Edge_Cuts, "Edges"),
]

for layer_info in layers:
    pctl.SetLayer(layer_info[1])
    pctl.OpenPlotfile(layer_info[0], pcbnew.PLOT_FORMAT_GERBER, layer_info[2])
    pctl.PlotLayer()
    pctl.ClosePlot()

##############################
# export drill

METRIC = True
ZERO_FORMAT = pcbnew.GENDRILL_WRITER_BASE.DECIMAL_FORMAT
INTEGER_DIGITS = 3
MANTISSA_DIGITS = 3
MIRROR_Y_AXIS = False
HEADER = True
OFFSET = pcbnew.wxPoint(0,0)
MERGE_PTH_NPTH = True
DRILL_FILE = True
MAP_FILE = False
REPORTER = None

drill_writer = pcbnew.EXCELLON_WRITER(board)
drill_writer.SetFormat(METRIC, ZERO_FORMAT, INTEGER_DIGITS, MANTISSA_DIGITS)
drill_writer.SetOptions(MIRROR_Y_AXIS, HEADER, OFFSET, MERGE_PTH_NPTH)
drill_writer.CreateDrillandMapFilesSet(gerber_dir, DRILL_FILE, MAP_FILE, REPORTER)

####################
# Create gerber zip
shutil.make_archive("{}/gerbers".format(output_dir), 'zip', root_dir=gerber_dir)
