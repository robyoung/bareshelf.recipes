"""add tags model

Revision ID: 5ccbad865613
Revises: 46b1bd044f5f
Create Date: 2020-04-27 14:06:39.108642

"""
from alembic import op
import sqlalchemy as sa


# revision identifiers, used by Alembic.
revision = "5ccbad865613"
down_revision = "46b1bd044f5f"
branch_labels = None
depends_on = None


def upgrade():
    op.create_table(
        "tags",
        sa.Column("slug", sa.String(), nullable=False),
        sa.Column("created_at", sa.DateTime(), nullable=True),
        sa.Column("updated_at", sa.DateTime(), nullable=True),
        sa.Column("id", sa.Integer(), autoincrement=True, nullable=False),
        sa.Column("name", sa.String(), nullable=False),
        sa.PrimaryKeyConstraint("id"),
    )
    op.create_index(op.f("ix_tags_slug"), "tags", ["slug"], unique=True)
    op.create_table(
        "ingredient_tags",
        sa.Column("tag_id", sa.Integer(), nullable=False),
        sa.Column("ingredient_id", sa.Integer(), nullable=False),
        sa.ForeignKeyConstraint(["ingredient_id"], ["ingredients.id"],),
        sa.ForeignKeyConstraint(["tag_id"], ["tags.id"],),
        sa.PrimaryKeyConstraint("tag_id", "ingredient_id"),
    )


def downgrade():
    op.drop_table("ingredient_tags")
    op.drop_index(op.f("ix_tags_slug"), table_name="tags")
    op.drop_table("tags")
